// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::collections::HashSet;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;
use crate::instruction_decoder::memory_reference_format_data_fields::{JmpFunction, ReferenceType};



pub(super) fn decode(instruction_word: u16, current_word_offset:u16, execution_context: Option<&mut ExecutionContext>) -> String {
    assert_eq!(get_bits(instruction_word,0,2),0);

    let function = JmpFunction::from(get_bits(instruction_word,3,4)as u8);
    let indirect = get_bits(instruction_word,5,5)== 1;
    let ref_type = ReferenceType::from(get_bits(instruction_word, 6, 7)as u8);
    let displacement_signed = get_bits(instruction_word,8,15) as i8;
    let displacement_unsigned = get_bits(instruction_word,8,15)as u8;

    //const FUNCTION_NAMES : [&'static str; 4] = ["JMP","JSR","ISZ","DSZ"];

    let mut ass_str = format!("{} ", function);

    if indirect{
        ass_str.push_str("@ ");
    }

    match ref_type {
        ReferenceType::Page0 => ass_str.push_str(format!("{:#x}", displacement_unsigned).as_str()),
        ReferenceType::PcRelative => {
            let result_adr = current_word_offset as i32 + displacement_signed as i32;
            ass_str.push_str(format!("{:+} ({:x})", displacement_signed,result_adr).as_str());
        },
        ReferenceType::AC2Based => ass_str.push_str(format!("AC2{:+}", displacement_signed).as_str()),
        ReferenceType::AC3Based => ass_str.push_str(format!("AC3{:+}", displacement_signed).as_str()),
    }

    if let Some(ec) = execution_context {

        let effective_adr = calculate_effective_adr(ec, ref_type, displacement_unsigned,indirect);
        
        match function {
            JmpFunction::JMP | JmpFunction::JSR =>{

                if function == JmpFunction::JSR{
                    ec.ac[3] = ec.ip + 1;
                }
                ec.ip = effective_adr;

                if function == JmpFunction::JSR{ // Update subroutine stats
                    ec.discovered_subroutines.entry("JSR".to_string()).and_modify(|x| {
                        x.insert(effective_adr);
                    }).or_insert(HashSet::from([effective_adr]));
                }


            }

            JmpFunction::DSZ | JmpFunction::ISZ => {
                let op_is_increment = function == JmpFunction::ISZ;

                let mut word:u16;
                
                word = ec.mapping_unit.read_word_from_memory(effective_adr,true);
                word = if op_is_increment { word.wrapping_add(1) } else { word.wrapping_sub(1)};
                ec.mapping_unit.write_word_to_memory(effective_adr, word,true);
             

                if word == 0{
                    ec.ip += 2;
                }else {
                    ec.ip += 1
                }
            }
        }

    }

    ass_str
}

pub(crate) fn calculate_effective_adr(ec : &mut ExecutionContext, reference_type: ReferenceType, displacement : u8, indirect:bool) -> u16{

    let displacement_signed= displacement as i8;
    let mut effective_adr = match reference_type {
        ReferenceType::Page0 => displacement as u16,
        ReferenceType::PcRelative =>(ec.ip as i16 + displacement_signed as i16) as u16,
        ReferenceType::AC2Based => (ec.ac[2] as i16 + displacement_signed as i16) as u16,
        ReferenceType::AC3Based => (ec.ac[3] as i16 + displacement_signed as i16) as u16,
    };

    ec.mapping_unit.use_instruction_map(reference_type == ReferenceType::PcRelative);

    if ec.ip == 0x2227{
        println!("BP");
    }

    if !ec.is_expanded_memory_for_the_current_user() {
        effective_adr &= 0x7fff;
    }
    
    if indirect {
        loop{
            let fetched_word = ec.mapping_unit.read_word_from_memory(effective_adr, true);
            let mut word = fetched_word;

            match effective_adr { // auto index check
                0o20..=0o27 => { // 0x10 - 0x17
                    //Indirect auto increment
                    word = word.wrapping_add(1);
                    ec.mapping_unit.write_word_to_memory(effective_adr, word, true);
                },
                0o30..=0o37 => {  // 0x18 - 0x1f
                    //Indirect auto decrement
                    word = word.wrapping_sub(1);
                    ec.mapping_unit.write_word_to_memory(effective_adr,word, true);
                }

                _ => {

                }
            } // match case

            effective_adr = word;

            if ec.is_expanded_memory_for_the_current_user() {
                break; // loop one time if expanded mem enabled
            }

            // Manual section 2.18: "When referencing auto-increment or auto-decrement locations,
            // the state of bit 0 BEFORE the increment or decrement operation is the condition which
            // is tested to determine whether or not to continue the indirect chain."
            //
            // So the continue/stop decision reads the word as it was fetched, while the address the
            // chain follows is the updated value. The two only differ on 0x7FFF -> 0x8000 through an
            // auto-increment location and 0x8000 -> 0x7FFF through an auto-decrement one, which is
            // exactly the boundary an addressing diagnostic will probe.
            //
            // OPEN QUESTION: in the 0x7FFF -> 0x8000 case the chain stops here, but the updated
            // value carries a bit 0 that section 2.17 says is the indirect bit rather than part of
            // the address. Whether the hardware hands back 0x8000 or masks it to 0x0000 is not
            // settled by the text. Left unmasked (the pre-existing behaviour); worth confirming on
            // the Memory Address Test tape before changing.
            if fetched_word & 0x8000 == 0{
                break; // if no indirect indicator break the loop
            }

            // else recurse one more to the address
            println!("Resolving multiple indirection");
            effective_adr = effective_adr & 0x7fff;

            // TODO (A7): manual section 1.6 faults a chain deeper than 16 levels. Without a depth
            // counter a self-referencing indirect word spins here forever.
        }

    } // end of the indirect case
    
    effective_adr
}

pub(crate) fn calculate_effective_adr_16bit_displacement(ec : &mut ExecutionContext, reference_type: ReferenceType, displacement : u16, indirect:bool) -> u16{

    let displacement_signed= displacement as i16;
    let mut effective_adr = match reference_type {
        ReferenceType::Page0 => displacement,
        ReferenceType::PcRelative =>(ec.ip as i16 + 1 + displacement_signed) as u16, // Dikkat bu instruction'da PC+1 den itibaren yükleniyor
        ReferenceType::AC2Based => (ec.ac[2] as i16 + displacement_signed) as u16,
        ReferenceType::AC3Based => (ec.ac[3] as i16 + displacement_signed) as u16,
    };

    ec.mapping_unit.use_instruction_map(reference_type == ReferenceType::PcRelative);
    
    if indirect {
        match effective_adr { // auto index check
            0o20..=0o27 => panic!("Indirect auto increment not implemented"),
            0o30..=0o37 => panic!("Indirect auto decrement not implemented"),
            _ => {}
        }
        effective_adr = ec.mapping_unit.read_word_from_memory(effective_adr,true);
    }

    effective_adr
}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use super::*;

    /// Manual section 2.18: "the state of bit 0 BEFORE the increment or decrement operation is the
    /// condition which is tested to determine whether or not to continue the indirect chain."
    ///
    /// Location 21 (octal) is an auto-increment location. Seeded with 0x7FFF, the word as FETCHED
    /// has bit 0 clear, so the chain must stop — even though the value written back and followed,
    /// 0x8000, has bit 0 set. Testing the updated word instead would take one more level of
    /// indirection, which the marker at location 0 makes visible.
    #[test]
    fn auto_index_tests_bit_zero_before_the_update() {
        let mut ec = ExecutionContext::new();
        let mut mem = vec![0u16; 0x40];
        mem[0] = 0x1234;                 // only reached if the chain wrongly continues
        mem[0o21] = 0x7fff;              // auto-increment location, bit 0 clear as fetched
        ec.load_initial_memory(mem);

        let efa = calculate_effective_adr(&mut ec, ReferenceType::Page0, 0o21, true);

        assert_eq!(ec.mapping_unit.read_word_from_memory(0o21, true), 0x8000,
                   "the auto-increment location is still updated");
        assert_ne!(efa, 0x1234,
                   "the chain must stop on the fetched bit 0, not the incremented one");
    }

    /// The ordinary case, to pin the behaviour the above test is contrasted against: a fetched word
    /// with bit 0 set does take another level.
    #[test]
    fn auto_index_continues_when_the_fetched_word_has_bit_zero_set() {
        let mut ec = ExecutionContext::new();
        let mut mem = vec![0u16; 0x40];
        mem[0o21] = 0x8030;              // bit 0 set -> continue
        mem[0x31]  = 0x002a;             // second level, reached via the INCREMENTED value 0x8031
        ec.load_initial_memory(mem);

        let efa = calculate_effective_adr(&mut ec, ReferenceType::Page0, 0o21, true);

        assert_eq!(ec.mapping_unit.read_word_from_memory(0o21, true), 0x8031,
                   "location incremented before the chain follows it");
        assert_eq!(efa, 0x002a, "one further level resolved");
    }

    #[test]
    fn test_01()  {
        let str = decode(0x517, 0, None);
        assert_eq!(str, "JMP @ +23 (17)");
    }

    #[test]
    fn test_unsigned_direct_page0_jmp()  {
        let mut ins_word = 0_u16;

        set_bits(&mut ins_word, 8, 15, 0xff);

        let mut ex = ExecutionContext::new();
        let str = decode(ins_word, 0, Some(&mut ex));
        assert_eq!(str, "JMP 0xff");
        assert_eq!(ex.ip, 0xff);

    }

    #[test]
    fn test_signed_relative_jsr()  {
        let mut ins_word = 0_u16;

        set_bits(&mut ins_word, 3, 4, 1); // JSR
        set_bits(&mut ins_word, 6, 7, 1); // Relative
        set_bits(&mut ins_word, 8, 15, 0xff); // Displacement

        let mut ex = ExecutionContext::new();
        ex.ip = 10;
        let str = decode(ins_word, 10, Some(&mut ex));
        assert_eq!(str, "JSR -1 (9)");
        assert_eq!(ex.ip, 9);
        assert_eq!(ex.ac[3], 11);

        ins_word = 0;
        set_bits(&mut ins_word, 3, 4, 0); // JMP
        set_bits(&mut ins_word, 6, 7, 3); // AC3 based
        set_bits(&mut ins_word, 8, 15, 0); // Displacement

        let str = decode(ins_word, 10, Some(&mut ex));
        assert_eq!(ex.ip, 11);
    }

    #[test]
    fn dzs_functional_test()  {

        let mut ec = ExecutionContext::new();

        let mut mem_vec = vec![0; 20];

        mem_vec[5] = 8; // pointer to 8
        mem_vec[8] = 1; // value to decrement

        ec.load_initial_memory(mem_vec);

        let mut ins_word = 0_u16;

        set_bits(&mut ins_word, 0, 4, 3); // DSZ
        set_bits(&mut ins_word, 5, 5, 1); // Indirect
        set_bits(&mut ins_word, 6, 7, 0); // Reference type page 0
        set_bits(&mut ins_word, 8, 15, 5); // Displacement

        let str = decode(ins_word, 0, Some(&mut ec));

        assert_eq!(ec.ip, 2);
        println!("{}",str);
        assert_eq!(str, "DSZ @ 0x5");

        //assert_eq!(ex.ac[3], 11);


    }
}
