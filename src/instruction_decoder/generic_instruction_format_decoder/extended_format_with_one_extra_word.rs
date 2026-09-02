// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::collections::HashSet;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::complex_instruction_executer::word_seach_fs;
use crate::virtual_machine::ExecutionContext;
use crate::instruction_decoder::memory_reference_format_data_fields::ReferenceType;
use crate::instruction_decoder::memory_reference_wo_acc_format_instruction_decoder::calculate_effective_adr_16bit_displacement;

pub(crate) const INS: [&str; 7] = ["IME","DME","PJSE","XCHM","LEF","JMPE","FS"];

pub(super) fn decode(mnemonic: &str, extra_word: u16, instruction_word: u16, ec: Option<&mut ExecutionContext>) -> String {

    let indirect = get_bits(instruction_word,5,5)== 1;
    let ref_type = ReferenceType::from(get_bits(instruction_word, 6, 7)as u8);

    let mut asm_string = format!("{} ", mnemonic);

    if indirect {
        asm_string.push_str("@ ");
    }

    match ref_type {
        ReferenceType::Page0 => asm_string.push_str(format!("{:#x} ", extra_word).as_str()),
        ReferenceType::PcRelative => asm_string.push_str(format!("{:+} ", extra_word).as_str()),
        ReferenceType::AC2Based => asm_string.push_str(format!("AC2{:+} ", extra_word).as_str()),
        ReferenceType::AC3Based => asm_string.push_str(format!("AC3{:+} ", extra_word).as_str()),
    }

    if let Some(ec) = ec {

        let effective_adr = calculate_effective_adr_16bit_displacement(ec, ref_type, extra_word,indirect);

        let mut auto_increment_ip = true;

        match mnemonic {
            "IME" => {
                // INCREMENT MEMORY, EXTENDED
                let word = ec.mapping_unit.read_word_from_memory(effective_adr,true);
                ec.mapping_unit.write_word_to_memory(effective_adr, word.wrapping_add(1),true);

            }

            "DME" => {
                // DECREMENT MEMORY, EXTENDED
                let word = ec.mapping_unit.read_word_from_memory(effective_adr,true);
                ec.mapping_unit.write_word_to_memory(effective_adr, word - 1,true);

            }

            "PJSE" => {
                println!("Branching to subroutine via PJSE instruction : {:#x}, from IP : {:#x}, call stack depth : {}",effective_adr,ec.ip,ec.call_stack_debug_information.len()+1);
                ec.push_a_single_word_to_the_stack(ec.ip + 2);

                ec.call_stack_debug_information.push_back(format!("Subroutine call via PJSE {:#x} from {:#x}",effective_adr,ec.ip));
                ec.ip = effective_adr;
                auto_increment_ip = false;
                //update subroutine stats
                ec.discovered_subroutines.entry("PJSE".to_string()).and_modify(|x| {
                    x.insert(effective_adr);
                }).or_insert(HashSet::from([effective_adr]));
            }

            "JMPE" => {
                println!("Jumping to subroutine via JMPE instruction : {:#x}, from IP : {:#x}",effective_adr,ec.ip);

                ec.ip = effective_adr;
                auto_increment_ip = false;

                //update subroutine stats
                ec.discovered_subroutines.entry("JMPE".to_string()).and_modify(|x| {
                    x.insert(effective_adr);
                }).or_insert(HashSet::from([effective_adr]));
            }

            "XCHM" => {
                // Exchange (ac) with the word specified by EFA.
                let ac_id = get_bits(instruction_word,3,4);
                let word = ec.mapping_unit.read_word_from_memory(effective_adr,true);

                ec.mapping_unit.write_word_to_memory(effective_adr ,  ec.ac[ac_id as usize],true);
                ec.ac[ac_id as usize] = word;

                asm_string.push_str(format!("AC{}",ac_id).as_str());
            }

            "LEF" => {
                let ac_id = get_bits(instruction_word,3,4);
                ec.ac[ac_id as usize] = effective_adr;
            }

            "FS" => {
                let search_result  = word_seach_fs(ec, extra_word);
                if let Some(adr)= search_result {
                    ec.ac[2] = adr;
                    ec.ip += 3;
                    auto_increment_ip = false;
                }else {
                    ec.ac[2] = ec.ac[3];
                    // Normal IP increment here
                }
            }




            _ => todo!("Unrecognized instruction mnemonic: {}", mnemonic)
        }

        if auto_increment_ip {
            ec.ip += 2;
        }

    }


    asm_string
}



#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use crate::instruction_decoder::generic_instruction_format_decoder;
    use super::*;

    #[test]
    fn lef_inst_test() {
        decode("LEF",0x814b,0x8008,None);
    }


}