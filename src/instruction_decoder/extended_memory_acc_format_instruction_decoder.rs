// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::{ACCUMULATOR_NAMES};
use crate::instruction_decoder::alc_format_instruction_executor::check_overflow;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;

pub(crate) const EXTENDED_MEMORY_TO_ACC_INS: [&str; 12] = ["LDFNA","ADFNA","SBFNA","ANFNA","LDFNX","ADFNX","SBFNX","ANFNX","LDFNW","ADFNW","SBFNW","ANFNW"];
pub(crate) const EXTENDED_ACC_TO_MEMORY_INS: [&str; 8] = ["STTNA","ADTNA","MGTNA","ANTNA","STTNX","ADTNX","MGTNX","ANTNX"];

pub(crate) fn decode(instruction_word: u16, extra_word: u16, execution_context: Option<&mut ExecutionContext>) -> String {
    assert_eq!(get_bits(instruction_word,0,2),3);

    let ac = get_bits(instruction_word,3,4);
    let function = get_bits(instruction_word,5,9);

    let mem_to_acc = get_bits(instruction_word,15,15) == 1;

    let function_name;
    if mem_to_acc {
        function_name = match function {
            0b00100 => "LDFNA",
            0b00101 => "ADFNA",
            0b00110 => "SBFNA",
            0b00111 => "ANFNA",
            0b01100 => "LDFNX",
            0b01101 => "ADFNX",
            0b01110 => "SBFNX",
            0b01111 => "ANFNX",
            0b10100 => "LDFNW",
            0b10101 => "ADFNW",
            0b10110 => "SBFNW",
            0b10111 => "ANFNW",
            _ => unreachable!()
        };
    }else{ // acc to memory than
        function_name = match function {
            0b00100 => "STTNA",
            0b00101 => "ADTNA",
            0b00110 => "MGTNA",
            0b00111 => "ANTNA",
            0b01100 => "STTNX",
            0b01101 => "ADTNX",
            0b01110 => "MGTNX",
            0b01111 => "ANTNX",
            _ => unreachable!(),
        }
    }


    let mut ass_str = String::from(function_name);
    ass_str.push_str(format!(" {}", ACCUMULATOR_NAMES[ac as usize]).as_str());
    ass_str.push_str(format!(" {:#x}", extra_word).as_str());

    if let Some(ec) = execution_context {
        
        match function_name {
            "LDFNW" => {
                ec.ac[ac as usize] = extra_word;
            }

            "SBFNW" => {
                let result= ec.ac[ac as usize].wrapping_sub(extra_word);
                ec.overflow_flag = check_overflow(extra_word,ec.ac[ac as usize],result,false);
                ec.ac[ac as usize] = result;
            }

            "SBFNA" => {
                let target_logical_adr  = extra_word;
                let operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);

                ec.ac[ac as usize] = ec.ac[ac as usize].wrapping_sub(operand);
                //TODO overflow

            }

            "SBFNX" => {
                let target_logical_adr  = ec.ac[2] + extra_word;
                let operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);

                ec.ac[ac as usize] = ec.ac[ac as usize].wrapping_sub(operand);
                //TODO overflow

            }

            "ADFNW" => {
                let result = ec.ac[ac as usize].wrapping_add(extra_word);
                ec.overflow_flag = check_overflow(ec.ac[ac as usize],extra_word,result,true);
                ec.ac[ac as usize] = result;
            }

            "ANFNW" => {
                ec.ac[ac as usize] &= extra_word;
            }

            "STTNA" => {
                let target_logical_adr  = extra_word;
                ec.mapping_unit.write_word_to_memory(target_logical_adr,ec.ac[ac as usize], true);
            }

            "STTNX" => {
                let target_logical_adr  = ec.ac[2] + extra_word;
                ec.mapping_unit.write_word_to_memory(target_logical_adr,ec.ac[ac as usize],true);
            }

            "ANTNA" => {
                // AND TO NEXT ADDRESS
                let target_logical_adr  = extra_word;
                let and_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                let and_result = ec.ac[ac as usize] & and_operand;
                ec.mapping_unit.write_word_to_memory(target_logical_adr, and_result,true);
            },

            "ANTNX" => {
                // AND TO NEXT ADDRESS, INDEXED BY AC2
                let target_logical_adr  = ec.ac[2] + extra_word;
                let and_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                let and_result = ec.ac[ac as usize] & and_operand;
                ec.mapping_unit.write_word_to_memory(target_logical_adr, and_result,true);
            }

            "ANFNA" => {
                // AND FROM NEXT ADDRESS
                let target_logical_adr  = extra_word;
                let and_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                ec.ac[ac as usize] &= and_operand;
            }

            "ANFNX" => {
                // AND FROM NEXT ADDRESS, INDEXED BY AC2
                let target_logical_adr  = ec.ac[2] + extra_word;
                let and_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                ec.ac[ac as usize] &= and_operand;
            }

            "ADFNA" => {
                //ADD FROM NEXT ADDRESS
                let target_logical_adr  = extra_word;
                let add_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                let result = ec.ac[ac as usize].wrapping_add(add_operand);
                ec.overflow_flag = check_overflow(ec.ac[ac as usize],add_operand,result,true);
                ec.ac[ac as usize] = result
            }

            "ADFNX" => {
                //ADD FROM NEXT ADDRESS, INDEXED BY AC2
                let target_logical_adr  = ec.ac[2].wrapping_add(extra_word);
                let add_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                ec.ac[ac as usize] = ec.ac[ac as usize].wrapping_add(add_operand);
                //TODO overflow check
            }

            "ADTNA" => {
                //ADD TO NEXT ADDRESS
                let target_logical_adr  = extra_word;
                let add_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                let add_result = ec.ac[ac as usize].wrapping_add(add_operand);
                ec.overflow_flag = check_overflow(ec.ac[ac as usize],add_operand,add_result,true);
                ec.mapping_unit.write_word_to_memory(target_logical_adr, add_result,true);

            }

            "ADTNX" => {
                //ADD TO NEXT ADDRESS INDEXED BY AC2
                let target_logical_adr  = ec.ac[2] + extra_word;
                let add_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                let add_result = ec.ac[ac as usize].wrapping_add(add_operand);
                ec.mapping_unit.write_word_to_memory(target_logical_adr, add_result,true);
                //TODO overflow check
            }

            "MGTNA" => {
                //MERGE TO NEXT ADDRESS
                let target_logical_adr  = extra_word;
                let change_mask_operand = ec.ac[1];
                let selected_bits_from_acc = ec.ac[ac as usize] & change_mask_operand;

                let source_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                let src_operand_bit_cleared = source_operand & !change_mask_operand;
                let result = selected_bits_from_acc | src_operand_bit_cleared;
                ec.mapping_unit.write_word_to_memory(target_logical_adr, result,true);
            }

            "MGTNX" => {
                //MERGE TO NEXT ADDRESS, INDEXED BY AC2
                let target_logical_adr  = ec.ac[2] + extra_word;
                let change_mask_operand = ec.ac[1];
                let selected_bits_from_acc = ec.ac[ac as usize] & change_mask_operand;

                let source_operand = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);
                let src_operand_bit_cleared = source_operand & !change_mask_operand;
                let result = selected_bits_from_acc | src_operand_bit_cleared;
                ec.mapping_unit.write_word_to_memory(target_logical_adr, result,true);
            }

            "LDFNX" => {
                let target_logical_adr  = ec.ac[2].wrapping_add(extra_word);
                ec.ac[ac as usize] = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);

            }

            "LDFNA" => {
                let target_logical_adr  = extra_word;
                ec.ac[ac as usize] = ec.mapping_unit.read_word_from_memory(target_logical_adr,true);

            }

            _ =>{
                todo!("Unimplemented extended mem acc format : {}", function_name);
            }
        }

        ec.ip += 2;
    }


    ass_str
}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use super::*;
    #[test]
    fn test_01()  {
        let mut val = 0_u16;
        set_bits(&mut val,0,2,3);
        set_bits(&mut val,5,9,0b10100);
        set_bits(&mut val,15,15,1);

        let mut ec = ExecutionContext::new();
        let str = decode(val, 0x1234_u16, Some(&mut ec));
        assert_eq!(str, "LDFNW AC0 0x1234");
        //println!("{:?}",ec);
    }

    #[test]
    fn extended_memory_mgtnx_test()  {

        let mut ec = ExecutionContext::new();

        let mut mem = vec![0_u16;10];
        mem[5] = 0x1234;

        ec.load_initial_memory(mem);


        ec.ac[0] = 0x5a; // donor
        ec.ac[1] = 0xff; // mask
        ec.ac[2] = 3; // AC2 indexed

        decode(0x6380, 2, Some(&mut ec)); // MGTNX AC0

        assert_eq!(ec.mapping_unit.read_word_from_memory(5,true), 0x125a);

        //println!("{}",ec)

    }

    #[test]
    fn extended_memory_load_store_operations_test()  {

        let mut ec = ExecutionContext::new();

        ec.load_initial_memory(vec![0;10]);


        decode(0x6501, 0x1234, Some(&mut ec)); // LDFNW AC0
        decode(0x6100, 5, Some(&mut ec)); // STTNA AC0 -> [5]
        assert_eq!(ec.mapping_unit.read_word_from_memory(5,true), 0x1234);

        ec.ac[0] = 0;
        ec.ac[2] = 3;

        decode(0x6301, 2, Some(&mut ec)); // LDFNX AC0, AC2 + 2
        decode(0x6300, 3, Some(&mut ec)); // STTNX AC0, AC2 + 3
        assert_eq!(ec.mapping_unit.read_word_from_memory(6,true), 0x1234);
        ec.ac[0] = 0;
        decode(0x6101, 6, Some(&mut ec)); // LDFNA AC0, [6]
        assert_eq!(ec.ac[0], 0x1234);



        //println!("{}",ec)

    }

}
