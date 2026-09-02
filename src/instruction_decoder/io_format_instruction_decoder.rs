// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::io_format_data_fields;
use crate::instruction_decoder::alc_format_data_fields::Accumulators;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::{io_device_emulator, ExecutionContext};
use crate::instruction_decoder::io_format_data_fields::Transfer;

const SPECIAL_IO_DEVICE_CODES: [u8;3] = [0,1,0x3f];



pub(super) fn decode(instruction_word: u16, execution_context: Option<&mut ExecutionContext>) -> String {
    assert_eq!(get_bits(instruction_word,0,2), 3);

    let acc = Accumulators::from(get_bits(instruction_word,3,4) as u8);
    let transfer = io_format_data_fields::Transfer::from(get_bits(instruction_word, 5, 7) as u8);
    let control = get_bits(instruction_word,8,9) as u8;
    let io_device = get_bits(instruction_word,10,15) as u8;


    let opcode_is_skip = transfer == Transfer::SKP;
    let opcode_is_no_io = transfer == Transfer::NIO;


    let mut ambiguous_instruction = false;

    if opcode_is_no_io && acc != Accumulators::AC0{
        eprintln!("NIO instruction with a accumulator specified, possible misinterpretation of higher instruction version");
        ambiguous_instruction = true;
    }
    if opcode_is_skip && acc != Accumulators::AC0{
        eprintln!("SKP instruction with a accumulator specified, possible misinterpretation of higher instruction version");
        ambiguous_instruction = true;
    }

    if SPECIAL_IO_DEVICE_CODES.contains(&io_device){
        eprintln!("IO Format with special IO device address, possible misinterpretation of higher instruction version");
        ambiguous_instruction = true;

    }


    let control_str = match control {
        0 => "",
        1 => "S",
        2 => "C",
        3 => "P",
        _ => unreachable!("control unrecognized")
    };

    let skip_suffix_str = match control {
        0 => "BN",
        1 => "BZ",
        2 => "DN",
        3 => "DZ",
        _ => unreachable!("control unrecognized")
    };



    let mut ass_str;
    if opcode_is_skip {
        ass_str = format!("{}{} {}", transfer, skip_suffix_str, io_device);
    }else if opcode_is_no_io {
        ass_str = format!("{}{}, {:#x}", transfer, control_str, io_device);
    }else{
        //Standard IO
         ass_str = format!("{}{} {}, {:#o}", transfer, control_str, acc, io_device);
    }


    if let Some(known_peripheral) = lookup_peripheral(io_device){
        ass_str.push_str(format!(" ({})", known_peripheral).as_str());
    }

    if ambiguous_instruction {
        ass_str.insert(0,'?');
    }

    if let Some(ec) = execution_context {

        if transfer.requires_data_transfer(){
            io_device_emulator::emulate_io_device(io_device, transfer.get_io_device_target_register(), transfer.is_read(), acc,  ec, lookup_peripheral(io_device));
        }
        if ass_str.contains("SKPDN"){
            ec.ip += 1;
        }
        ec.ip += 1;
    }

    ass_str
}



fn lookup_peripheral(device_code:u8) -> Option<&'static str> {

    match device_code {

        _ => None,
    }

}

#[cfg(test)]
mod tests {

    #[test]
    fn test_01(){
        let instruction_word = 0o61434_u16;
        let decoded_instruction = super::decode(instruction_word,None);
        assert_eq!(decoded_instruction,"DIB AC0, 0o34")
    }

    #[test]
    fn test_02(){
        let instruction_word = 0o63402;
        let decoded_instruction = super::decode(instruction_word,None);
        assert_eq!(decoded_instruction,"SKPBN 2")


    }

    #[test]
    fn test_03(){
        let instruction_word = 0o60100;
        let decoded_instruction = super::decode(instruction_word,None);
        assert_eq!(decoded_instruction,"?NIOS, 0x0")


    }

    #[test]
    fn ambiguous_instruction_test_01(){
        let instruction_word = 0x68A0; // LASH instruction
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn ambiguous_instruction_test_02(){
        let instruction_word = 0x7040; // FLDS instruction
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn ambiguous_instruction_test_03(){
        let instruction_word = 0o60177; // INTEN instruction
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn ambiguous_instruction_test_04(){
        let instruction_word = 0o70102; // FLDS instruction (floating point)
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn test_instruction_scratch_pad(){
        let instruction_word = 0x68A0;
        let decoded_instruction = super::decode(instruction_word,None);
        println!("{:?}", decoded_instruction);


    }
}