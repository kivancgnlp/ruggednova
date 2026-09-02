// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

mod alc_format_instruction_builder;
mod common_assembler_utils;

use std::collections::HashMap;
use std::io::Error;
use crate::instruction_decoder::bit_utils::set_bits;
use crate::instruction_identifier::instruction_data_parser::InstructionData;
use crate::instruction_identifier::InstructionIdentifier;


pub(crate) struct Assembler{
    mnemonic_map : HashMap<String, InstructionData>,
}

pub(crate) fn parse_hex_string(hex_string : &str) -> Option<u16> {
    let x_place = hex_string.find(|c|{ c == 'x' || c == 'X'})?;
    let extra_word  =u16::from_str_radix(&hex_string[x_place+1..], 16).ok()?;
    Some(extra_word)
}

impl Assembler {

    pub fn new() -> Result<Assembler, std::io::Error> {
        let instruction_identifier = InstructionIdentifier::new()?;
        Self::build_mnemonic_map(&instruction_identifier)
    }

    pub fn new_using_instruction_identifier(instruction_identifer: &InstructionIdentifier) -> Result<Assembler, Error> {
        Self::build_mnemonic_map(&instruction_identifer)
    }

    fn build_mnemonic_map(instruction_identifer: &InstructionIdentifier) -> Result<Assembler, Error> {
        let mut mnemonic_map = HashMap::new();

        instruction_identifer.get_instructions().iter().for_each(|i| {
            if mnemonic_map.contains_key(&i.mnemonic) {
                eprintln!("Duplicate mnemonic");
            }
            mnemonic_map.insert(i.mnemonic.clone(), i.clone());
        });
        Ok(Assembler { mnemonic_map })
    }

    pub(crate) fn assemble_lines(&self, asm_lines : &str) -> Vec<u16>{

        let mut ins_word_buffer = Vec::new();

        for line in asm_lines.lines() {

            let mut line = line.trim();

            if let Some(comment_start) = line.find(';') {
                line = &line[..comment_start]; // Trim comment if present
            }

            let asm_result = self.assemble_line(line);

            match asm_result {
                Some(line_words) => {
                    line_words.iter().for_each(|word| ins_word_buffer.push(*word));
                }
                None => {
                    eprintln!("Failed to assemble line {}",line);
                    break;
                }
            }
        }

        ins_word_buffer

    }



    pub(crate) fn assemble_line(&self, ass_src_line: &str) -> Option<Vec<u16>> {
        let mnemonic_part = ass_src_line.split_whitespace().nth(0)?;

        let instruction_data_query_result = self.mnemonic_map.get(mnemonic_part);

        if instruction_data_query_result.is_none() {

            let mut split_ws = ass_src_line.split_whitespace();

            let first_part = split_ws.next()?;


            if first_part == "DW"{ // Uydurduğum bazı özel assembly (Define Word)
                let data_part = split_ws.next()?;

                let data_word = parse_hex_string(data_part)?;

                return Some(vec![data_word]);

            }

            if first_part == "DUPW"{ // Uydurduğum bazı özel assembly (Duplicate Word)
                let data_part = split_ws.next()?;

                let duplicate_word_count = parse_hex_string(data_part)?;


                return Some(vec![0; duplicate_word_count as usize]);

            }

            return None;// No match case ( Neither custom nor mnemonic match found)

        }


        let instruction_data = instruction_data_query_result.unwrap();
        //println!("Base code : {:x}", instruction_data.match_value);

        if instruction_data.base_type == "ALC" {
            let ins_word = alc_format_instruction_builder::build(ass_src_line, instruction_data.match_value)?;
            let ins_words = vec![ins_word];
            return Some(ins_words);
        }

        const ONE_ACC_ONE_EXTRA_WORD_INSTRUCTIONS : [&str;4] = ["LDFNW","LDFNA","STTNA","STTNX"];

        if ONE_ACC_ONE_EXTRA_WORD_INSTRUCTIONS.contains(&mnemonic_part){
            let ac_part = ass_src_line.split_whitespace().nth(1)?;
            let ac = common_assembler_utils::parse_accumulator(ac_part)?;

            let mut ins_word = instruction_data.match_value;
            set_bits(&mut ins_word, 3, 4, ac as u8 as u16);

            let immediate_part = ass_src_line.split_whitespace().nth(2)?;
            let extra_word  = parse_hex_string(immediate_part)?;

            let ins_words = vec![ins_word,extra_word];
            return Some(ins_words);
        }

        const JUST_ONE_ACC_INSTRUCTIONS : [&str;2] = ["WMSR","PSH"];

        if JUST_ONE_ACC_INSTRUCTIONS.contains(&mnemonic_part){
            let ac_part = ass_src_line.split_whitespace().nth(1)?;
            let ac = common_assembler_utils::parse_accumulator(ac_part)?;

            let mut ins_word = instruction_data.match_value;
            set_bits(&mut ins_word, 3, 4, ac as u8 as u16);


            let ins_words = vec![ins_word];
            return Some(ins_words);
        }

        if mnemonic_part == "JMP"{

            let mut ins_word = instruction_data.match_value;

            let mut pc_releative = false;
            if ass_src_line.contains('-') || ass_src_line.contains('+'){
                pc_releative = true;
            }

            let mut disp : u8;
            if pc_releative{
                set_bits(&mut ins_word, 7, 7, 1);
                disp = u8::from_str_radix(ass_src_line.split_whitespace().nth(1).unwrap().get(1..)?, 16).ok()?;
                if ass_src_line.contains('-'){ //TODO : sonra daha iyi yapılabilir
                    disp = -(disp as i8) as u8;
                }
                set_bits(&mut ins_word, 8, 15, disp as u16);
            } else {
                disp = u8::from_str_radix(ass_src_line.split_whitespace().nth(1).unwrap().get(1..)?, 16).ok()?;
                set_bits(&mut ins_word, 8, 15, disp as u16);
            }

            let ins_words = vec![ins_word];
            return Some(ins_words);
        }





        const NO_ARG_OPERANDS:[&str;8]= ["HALT","ECALL","WRWRD","STEM","RTFNI","EXMAP","DXMAP","MAPSI"];

        if NO_ARG_OPERANDS.contains(&mnemonic_part){
            let ins_words = vec![instruction_data.match_value];
            return Some(ins_words);
        }

        eprintln!("Couldn't assemble mnemonic {}",mnemonic_part);
        None
    }


}


#[cfg(test)]
mod tests {
    use std::io::stdin;
    use crate::assembler::Assembler;
    use crate::instruction_decoder;

    #[test]
    fn alc_ins_test_01() -> Result<(), std::io::Error> {

        let assembler = Assembler::new()?;

        let test_asm_line = "INC OR# AC0,AC1 SNR";
        let ins = assembler.assemble_line(test_asm_line).ok_or(std::io::Error::other(""))?;

        println!("{:#x}", ins[0]);

        let re_asm = instruction_decoder::alc_format_instruction_decoder::decode(ins[0], None);
        println!("{}", re_asm);

        let re_asm_without_paren = re_asm.get(0..re_asm.find('(').unwrap()).ok_or(std::io::Error::other(""))?;
        if test_asm_line == re_asm_without_paren{
            println!("Instruction encoded and decoded successfully");
        }
        Ok(())

    }

    #[test]
    fn ldfnw_functional_test() -> Result<(), std::io::Error> {

        let assembler = Assembler::new()?;

        let test_asm_line = "LDFNW AC1 0x1234";
        let ins = assembler.assemble_line(test_asm_line).ok_or(std::io::Error::other(""))?;

        println!("{:#x} {:#x}", ins[0], ins[1]);

        let mut ex = crate::virtual_machine::ExecutionContext::new();
        let re_asm = instruction_decoder::extended_memory_acc_format_instruction_decoder::decode(ins[0],ins[1], Some(&mut ex));
        println!("{}", re_asm);
        assert_eq!(test_asm_line, re_asm);
        assert_eq!(ex.ac[1], 0x1234);


        Ok(())

    }

    #[test]
    #[ignore] // Not include in all tests
    fn test_assembler_from_cmdline() -> Result<(), std::io::Error> {

        let assembler = Assembler::new()?;

        let mut input_line = String::new();

        loop{
            input_line.clear();
            stdin().read_line(&mut input_line)?;
            println!("got a line: {}", input_line);
            if input_line.starts_with("q"){
                break;
            }
            let ins = assembler.assemble_line(input_line.as_str()).ok_or(std::io::Error::other("Assemble error"))?;

            println!("{:#x}", ins[0]);
            let re_asm = instruction_decoder::alc_format_instruction_decoder::decode(ins[0], None);
            println!("{}", re_asm);
        }


        Ok(())

    }
}
