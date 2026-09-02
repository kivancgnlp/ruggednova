// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::collections::HashSet;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;
use crate::instruction_decoder::memory_reference_format_data_fields::ReferenceType;
use crate::instruction_decoder::memory_reference_wo_acc_format_instruction_decoder::calculate_effective_adr_16bit_displacement;

pub(crate) const INS: [&str; 11] = ["PJS","SAVE","LDAE","STAE","DST","DLD","DLDX","DSTX","DNA","DNAX","DAD"];

pub(super) fn decode(mnemonic: &str, extra_word: u16, instruction_word: u16, ec: Option<&mut ExecutionContext>) -> String {

    let asm_string = match mnemonic{
        "LDAE" | "STAE" => {
            let ac = get_bits(instruction_word, 3, 4);
            let indirect = get_bits(instruction_word, 5, 5) == 1;
            let ref_type = ReferenceType::from(get_bits(instruction_word, 6, 7) as u8);
            let displacement = extra_word;
            format!("{} AC{} [{} {:?} {:#x}]", mnemonic, ac, if indirect{'@'} else {' '}, ref_type,extra_word)

        }
        _ => format!("{} {:#x}", mnemonic, extra_word) // This is default asm string but LDAE and STAE need some more info
    };

    if let Some(ec) = ec {

        let mut auto_increment_ip = true;

        match mnemonic {
            "PJS" => {
                println!("Branching to subroutine via PJS instruction : {:#x}, from IP : {:#x}, call stack depth : {}",extra_word,ec.ip,ec.call_stack_debug_information.len()+1);
                ec.push_a_single_word_to_the_stack(ec.ip + 2);

                ec.call_stack_debug_information.push_back(format!("Subroutine call via PJS {:#x} from {:#x}",extra_word,ec.ip));
                ec.ip = extra_word;
                auto_increment_ip = false;

                //update subroutine stats
                ec.discovered_subroutines.entry("PJS".to_string()).and_modify(|x| {
                    x.insert(extra_word);
                }).or_insert(HashSet::from([extra_word]));
            }

            "SAVE" => {
                ec.push_a_single_word_to_the_stack(ec.ac[3]);
                ec.push_a_single_word_to_the_stack(ec.ac[2]);
                ec.push_a_single_word_to_the_stack(ec.ac[1]);
                ec.push_a_single_word_to_the_stack(ec.ac[0]);
                ec.push_a_single_word_to_the_stack(ec.encode_carry_and_overflow());
                ec.push_a_single_word_to_the_stack(ec.fp);

                ec.fp = ec.sp;

                for i in 0..extra_word{
                    ec.push_a_single_word_to_the_stack(0xaacc); // Boyle yapmak kolay geldi, ek özellik stack canary de olur
                }

                ec.ac[3] = ec.fp;

            }

            "LDAE" | "STAE" => { // Similar to LDA instruction but displacement is 16 bit
                let ac = get_bits(instruction_word,3,4);
                let indirect = get_bits(instruction_word,5,5)== 1;
                let ref_type = ReferenceType::from(get_bits(instruction_word, 6, 7)as u8);
                let displacement = extra_word;

                let effective_address = calculate_effective_adr_16bit_displacement(ec, ref_type, displacement, indirect);


                let load_acc_op = mnemonic == "LDAE";

                if load_acc_op {
                    // Load ACC operation
                    let word= ec.mapping_unit.read_word_from_memory(effective_address,true);
                    
                    ec.ac[ac as usize] = word;
                }else {
                    // Store ACC operation
                    
                    ec.mapping_unit.write_word_to_memory(effective_address,ec.ac[ac as usize], true);
                 
                }

            },
            "DST" => {
                // Double store
                let ac01 = ec.get_ac01_compound();
                ec.mapping_unit.write_double_word_to_data_memory(extra_word,ac01);
            },

            "DSTX" =>{
                // DOUBLE STORE, INDEXED BY AC2
                let address = extra_word + ec.ac[2];
                let ac01 = ec.get_ac01_compound();
                ec.mapping_unit.write_double_word_to_data_memory(address,ac01);

            },

            "DLD" => {
                // Double load
                let dw = ec.mapping_unit.read_double_word_from_memory(extra_word);
                ec.set_ac01_compound(dw);
            }

            "DLDX" => {
                // DOUBLE LOAD, INDEXED BY AC2
                let address = extra_word + ec.ac[2];
                let dw = ec.mapping_unit.read_double_word_from_memory(address);
                ec.set_ac01_compound(dw);
            }

            "DNA" => {
                //DOUBLE NEGATE AND ADD
                // DW çıkarma işlemi
                let ac01 = ec.get_ac01_compound() as i32;

                let op_dw = ec.mapping_unit.read_double_word_from_memory(extra_word)as i32;

                let result = op_dw - ac01;
                ec.set_ac01_compound(result as u32);
            }

            "DNAX" => {
                //DOUBLE NEGATE AND ADD, INDEXED BY AC2
                // DW çıkarma işlemi
                let ac01 = ec.get_ac01_compound() as i32;

                let target_address = ec.ac[2] + extra_word;
                let op_dw = ec.mapping_unit.read_double_word_from_memory(target_address)as i32;

                let result = op_dw - ac01;
                ec.set_ac01_compound(result as u32);
            }

            "DAD" => {
                // DW toplama işlemi
                let ac01 = ec.get_ac01_compound() ;

                let op_dw = ec.mapping_unit.read_double_word_from_memory(extra_word);

                let result = op_dw + ac01;
                ec.set_ac01_compound(result);
            }


            _ => todo!("Unrecognized instruction mnemonic: {}", mnemonic)
        }

        if auto_increment_ip{
            ec.ip += 2;
        }
    }


    asm_string
}



#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use super::*;

    #[test]
    fn dna_test()  {

        let mut ec = ExecutionContext::new();

        let mut vec1 = vec![0; 20];

        ec.ac[0] = 0;
        ec.ac[1] = 1;
        vec1[6] = 0x8002;
        vec1[7] = 0;
        ec.load_initial_memory(vec1);

        decode("DNA", 6, 0, Some(&mut ec));

        assert_eq!(ec.ip,2);
        assert_eq!(ec.get_ac01_compound(),0x8001ffff); // fixed 10-02-2026

        println!("{:#x}",ec.get_ac01_compound())

    }
    #[test]
    fn dad_test()  {

        let mut ec = ExecutionContext::new();

        let mut vec1 = vec![0; 20];

        ec.ac[0] = 0;
        ec.ac[1] = 1;
        vec1[6] = 0x8001;
        vec1[7] = 0xffff;
        ec.load_initial_memory(vec1);

        decode("DAD", 6, 0, Some(&mut ec));
        assert_eq!(ec.ac[0],0x8002);
        assert_eq!(ec.ac[1],0);
        assert_eq!(ec.ip,2);

        println!("{}",ec)

    }

    #[test]
    fn subroutine_call_and_return_test()  {

        let mut ec = ExecutionContext::new();

        ec.load_initial_memory(vec![0;20]);

        ec.fp = 5;
        ec.sp = 20;
        ec.ip = 7;
        decode("PJS", 15, 0, Some(&mut ec)); // PJS 2

        crate::instruction_decoder::generic_instruction_format_decoder::decode("PRT", 0, Some(&0_u16), Some(&mut ec));

        assert_eq!(ec.ip, 9);
        assert_eq!(ec.sp, 20);

        println!("{}",ec)

    }

    #[test]
    fn subroutine_call_and_return_test_with_jsr_and_popb()  {

        let mut ec = ExecutionContext::new();

        ec.load_initial_memory(vec![0;20]);

        ec.ac[0] = 1;
        ec.ac[1] = 2;
        ec.ac[2] = 3;
        ec.ac[3] = 4;
        ec.carry_flag = true;
        ec.fp = 5;
        ec.sp = 20;
        ec.ip = 7;

        let mut jsr_ins_word = 0_u16;

        set_bits(&mut jsr_ins_word, 3, 4, 1); // JSR
        set_bits(&mut jsr_ins_word, 6, 7, 0); // Page 0
        set_bits(&mut jsr_ins_word, 8, 15, 15); // Displacement

        crate::instruction_decoder::memory_reference_wo_acc_format_instruction_decoder::decode(jsr_ins_word,0,Some(&mut ec));
        decode("SAVE", 3,0, Some(&mut ec)); // SAVE 3

        // Try to use some place from the frame we allocated
        ec.mapping_unit.write_word_to_memory(ec.ac[3] - 1 , 0x10,true);
        ec.mapping_unit.write_word_to_memory(ec.ac[3] - 2 , 0x11,true);
        ec.mapping_unit.write_word_to_memory(ec.ac[3] - 3 , 0x12,true);

        assert_eq!(ec.ip, 17);

        ec.ac[0] = 0;
        ec.ac[1] = 0;
        ec.ac[2] = 0;
        ec.ac[3] = 0;
        ec.carry_flag = false;

        crate::instruction_decoder::generic_instruction_format_decoder::decode("POPB", 0, Some(&0_u16), Some(&mut ec));

        let mut ins_word = 0_u16;
        set_bits(&mut ins_word, 0, 4, 0); // JMP
        set_bits(&mut ins_word, 6, 7, 3); // AC3 based
        set_bits(&mut ins_word, 8, 15, 0); // Displacement
        crate::instruction_decoder::memory_reference_wo_acc_format_instruction_decoder::decode(ins_word, 0, Some(&mut ec));

        assert_eq!(ec.ac[0], 1);
        assert_eq!(ec.ac[1], 2);
        assert_eq!(ec.ac[2], 3);
        // Do not compare AC3 since it is link register in this case
        //assert_eq!(ec.ac[3], 4);
        assert_eq!(ec.carry_flag, true);

        assert_eq!(ec.ip, 8);
        assert_eq!(ec.sp, 20);

        println!("{}",ec)

    }


    #[test]
    fn subroutine_call_and_return_test_with_prt_and_rtrn()  {

        let mut ec = ExecutionContext::new();

        ec.load_initial_memory(vec![0;20]);

        ec.ac[0] = 1;
        ec.ac[1] = 2;
        ec.ac[2] = 3;
        ec.ac[3] = 4;
        ec.carry_flag = true;
        ec.fp = 5;
        ec.sp = 20;
        ec.ip = 7;

        decode("PJS", 15,0, Some(&mut ec)); // PJS 15
        decode("SAVE", 3,0, Some(&mut ec)); // SAVE 3

        // Try to use some place from the frame we allocated
        ec.mapping_unit.write_word_to_memory(ec.ac[3] - 1 , 0x10,true);
        ec.mapping_unit.write_word_to_memory(ec.ac[3] - 2 , 0x11,true);
        ec.mapping_unit.write_word_to_memory(ec.ac[3] - 3 , 0x12,true);

        assert_eq!(ec.ip, 17);
        decode("PJS", 23,0, Some(&mut ec)); // PJS 23
        crate::instruction_decoder::generic_instruction_format_decoder::decode("PRT", 0,Some(&0_u16), Some(&mut ec)); // PRT return
        assert_eq!(ec.ip, 19);

        ec.ac[0] = 0;
        ec.ac[1] = 0;
        ec.ac[2] = 0;
        ec.ac[3] = 0;
        ec.carry_flag = false;

        crate::instruction_decoder::generic_instruction_format_decoder::decode("RTRN", 0, Some(&0_u16), Some(&mut ec));


        assert_eq!(ec.ac[0], 1);
        assert_eq!(ec.ac[1], 2);
        assert_eq!(ec.ac[2], 3);
        assert_eq!(ec.ac[3], 4);
        assert_eq!(ec.carry_flag, true);

        assert_eq!(ec.ip, 9);
        assert_eq!(ec.sp, 20);

        println!("{}",ec)

    }

}