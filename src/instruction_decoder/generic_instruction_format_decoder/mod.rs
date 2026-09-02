// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::virtual_machine::ExecutionContext;

mod generic_format_with_just_acc_no_extra_word;
pub(crate) mod generic_format_with_two_acc_for_byte_ops_wo_extra_word;
mod generic_format_with_immediate_op_for_shifts_wo_extra_word;
mod generic_format_with_one_extra_word;
pub(crate) mod complex_instruction_info;
mod generic_format_with_two_acc_wo_extra_word;
mod extended_format_with_one_extra_word;

pub(crate) fn decode(instruction_mnemonic : &str, instruction_word : u16, extra_instruction_word : Option<&u16>, ec: Option<&mut ExecutionContext>) -> Option<String>{

    let mut decoded = "".to_string();
    let mut unable = false;
    if generic_format_with_just_acc_no_extra_word::INS.contains(&instruction_mnemonic){
        decoded = generic_format_with_just_acc_no_extra_word::decode(instruction_mnemonic,instruction_word,ec);
    }else if generic_format_with_two_acc_for_byte_ops_wo_extra_word::INS.contains(&instruction_mnemonic){
        decoded = generic_format_with_two_acc_for_byte_ops_wo_extra_word::decode(instruction_mnemonic,instruction_word,ec);
    }else if generic_format_with_immediate_op_for_shifts_wo_extra_word::B3_INS.contains(&instruction_mnemonic){
        decoded = generic_format_with_immediate_op_for_shifts_wo_extra_word::decode_3bit(instruction_mnemonic,instruction_word,ec);
    }else if generic_format_with_immediate_op_for_shifts_wo_extra_word::B4_INS.contains(&instruction_mnemonic) {
        decoded = generic_format_with_immediate_op_for_shifts_wo_extra_word::decode_4bit(instruction_mnemonic, instruction_word,ec);
    }else if generic_format_with_immediate_op_for_shifts_wo_extra_word::B6_INS.contains(&instruction_mnemonic) {
        decoded = generic_format_with_immediate_op_for_shifts_wo_extra_word::decode_6bit(instruction_mnemonic, instruction_word,ec);
    } else if generic_format_with_one_extra_word::INS.contains(&instruction_mnemonic) {
        decoded = generic_format_with_one_extra_word::decode(instruction_mnemonic, *extra_instruction_word.unwrap(),instruction_word,ec);
    } else if generic_format_with_two_acc_wo_extra_word::INS.contains(&instruction_mnemonic) {
        decoded = generic_format_with_two_acc_wo_extra_word::decode(instruction_mnemonic,instruction_word,ec);
    } else if extended_format_with_one_extra_word::INS.contains(&instruction_mnemonic) {
        decoded = extended_format_with_one_extra_word::decode(instruction_mnemonic,*extra_instruction_word.unwrap(),instruction_word,ec);
    } else {
        if let Some(d) = complex_instruction_info::explain(instruction_mnemonic, ec){
            decoded = d;
        }else {
            unable = true;
        }
        
    }

    if !unable {
        return Some(decoded);
    }

    None
}