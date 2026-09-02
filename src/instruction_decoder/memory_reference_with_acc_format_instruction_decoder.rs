// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::ACCUMULATOR_NAMES;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;
use crate::instruction_decoder::memory_reference_format_data_fields::ReferenceType;


pub(super) fn decode(instruction_word: u16, current_word_offset:u16, execution_context: Option<&mut ExecutionContext>) -> String {
    assert_eq!(get_bits(instruction_word,0,0),0);

    let function = get_bits(instruction_word,1,2);
    let ac = get_bits(instruction_word,3,4);
    let indirect = get_bits(instruction_word,5,5)== 1;
    let ref_type = ReferenceType::from(get_bits(instruction_word, 6, 7)as u8);
    let displacement_signed = get_bits(instruction_word,8,15) as i8;
    let displacement_unsigned = get_bits(instruction_word,8,15)as u8;

    let function_name = match function {
        1 => "LDA",
        2 => "STA",
        _ => unreachable!()
    };

    let mut ass_str = String::from(function_name);
    ass_str.push_str(" ");

    ass_str.push_str(ACCUMULATOR_NAMES[ac as usize]);
    ass_str.push_str(",[");

    if indirect{
        ass_str.push_str("@ ");
        ass_str.push_str(" ");
    }

    match &ref_type {
        ReferenceType::Page0 => ass_str.push_str(format!("{:#x}", displacement_unsigned).as_str()),
        ReferenceType::PcRelative => {
            //ass_str.push_str(format!("{:+}", displacement_signed).as_str())
            let result_adr = current_word_offset as i32 + displacement_signed as i32;
            ass_str.push_str(format!("{:+} ({:x})", displacement_signed,result_adr).as_str())
        },
        ReferenceType::AC2Based => ass_str.push_str(format!("AC2{:+}", displacement_signed).as_str()),
        ReferenceType::AC3Based => ass_str.push_str(format!("AC3{:+}", displacement_signed).as_str()),

    }

    if let Some(ec) = execution_context {

        let effective_address = crate::instruction_decoder::memory_reference_wo_acc_format_instruction_decoder::calculate_effective_adr(ec, ref_type, displacement_unsigned,indirect);

        
        match function_name {
            "LDA" => {
                let word= ec.mapping_unit.read_word_from_memory(effective_address,true);
                
                ec.ac[ac as usize] = word;
            },
            "STA" => {

                ec.mapping_unit.write_word_to_memory(effective_address,ec.ac[ac as usize],true);
                
             
            },
            _ => unreachable!()
        }
        ec.ip += 1;
    }

    ass_str.push_str("]");
    ass_str
}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use super::*;
    #[test]
    fn test_01()  {
        let mut val = 0_u16;
        set_bits(&mut val,1,2,1);
        let str = decode(val, 0, None);
        assert_eq!(str, "LDA AC0,[0x0]");
    }

}
