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
            //let word_addressed = effective_adr;
            let mut word = ec.mapping_unit.read_word_from_memory(effective_adr, true);

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

            if effective_adr & 0x8000 == 0{
                break; // if no indirect indicator break the loop
            }

            // else recurse one more to the address
            println!("Resolving multiple indirection");
            effective_adr = effective_adr & 0x7fff;

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
