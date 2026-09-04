// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::read_dir;
use std::io::{BufReader, Error, Read, Result};
use std::path::Path;
use xml::EventReader;
use xml::reader::XmlEvent;

#[derive(Debug,Clone)]
pub(crate) struct InstructionData{
    pub match_value:u16,
    pub match_mask:u16,
    pub following_word_count:u8,
    pub(crate) mnemonic:String,
    pub base_type:String,
    pub parse_status:u8,
}

impl InstructionData{
    pub fn new() -> InstructionData{
        InstructionData{
            match_value: 0,
            match_mask: 0,
            following_word_count: 0,
            mnemonic: "".to_string(),
            base_type: "".to_string(),
            parse_status: 0,
        }
    }
}

impl Display for InstructionData{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error>{
        write!(f,"{:<6} match_value : {:#x} ({:#o}) \t match_mask : {:#x} \t following_word_count {}\t base_type : {} ",self.mnemonic, self.match_value,self.match_value, self.match_mask,self.following_word_count,self.base_type)
    }
}

pub(crate) fn parse_instruction_informations() -> Result<Vec<InstructionData>>{

    let mut all_instructions: Vec<InstructionData> = Vec::new();

    let path = Path::new("Data/Instruction_Informations/");

    for file in read_dir(path)?{
        if let Ok(file) = file{
            if file.path().is_file() && file.path().extension() == Some(std::ffi::OsStr::new("xml")){
                let input_file = fs::File::open(file.path())?;
                let buff_reader = BufReader::new(input_file);
                //println!("Parsing file : {}, instruction count : {}", file.path().display(), all_instructions.len());
                parse_file_instructions(buff_reader, &mut all_instructions)?;
            }
        }

    }

    println!("{} instruction information parsed",all_instructions.len());


    Ok(all_instructions)

}

fn parse_file_instructions(buff_reader : impl Read, instructions : &mut Vec<InstructionData>) -> Result<()>{

    let parser = EventReader::new(buff_reader);

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name: element, attributes, .. }) => {
                if element.local_name == "instruction"{
                    let mut instruction_data = InstructionData::new();

                    for attr in attributes {

                        //println!("  Attribute: {} = {}", attr.name.local_name, attr.value);
                        match attr.name.local_name.as_str(){
                            "match_value" => {
                                instruction_data.match_value = u16::from_str_radix(attr.value.as_str(),16).map_err(|e| Error::other(e))?;
                                instruction_data.parse_status |= 1;
                            }
                            "match_mask" => {
                                instruction_data.match_mask = u16::from_str_radix(attr.value.as_str(),16).map_err(|e| Error::other(e))?;
                                instruction_data.parse_status |= 2;
                            }
                            "following_word_count" => {
                                instruction_data.following_word_count = u8::from_str_radix(attr.value.as_str(),16).map_err(|e| Error::other(e))?;
                                instruction_data.parse_status |= 4;
                            }

                            "mnemonic" => {
                                instruction_data.mnemonic = attr.value.clone();
                                instruction_data.parse_status |= 8;
                            }
                            "base_type" => {
                                instruction_data.base_type = attr.value.clone();
                                instruction_data.parse_status |= 0x10;
                            }

                            "match_value_octal" => { // Only used for document referencing
                                let match_value_from_octal = u16::from_str_radix(attr.value.as_str(), 8).map_err(|e| Error::other(e))?;

                                if match_value_from_octal != instruction_data.match_value{
                                    eprintln!("Mismatched octal value for instruction : {}", instruction_data.mnemonic);
                                }
                            }
                            _=>{
                                eprintln!("Unknown Attribute: {}", attr.name.local_name)
                            }
                        }

                    };

                    if instruction_data.parse_status == 0x1f{
                        instructions.push(instruction_data);
                    }else {
                        eprintln!("Instruction all fields not parsed: {:#x}", instruction_data.parse_status);
                    }
                }else{
                    if element.local_name != "root"{
                        eprintln!("Unknow element: {}", element.local_name);
                    }

                }

            }
            //Ok(XmlEvent::Characters(s)) => println!("Text: {}", s),
            //Ok(XmlEvent::EndElement { name }) => println!("End: {}", name),
            //Ok(XmlEvent::Whitespace(s)) => println!("Whitespace: {}", s),
            _ => {
                //println!("{:?}", e);
            }
        }
    }

    Ok(())

}

#[cfg(test)]
mod tests{
    #[test]
    fn filter_instructions_01(){
        let instruction = super::parse_instruction_informations().unwrap();

        instruction.iter().filter(|x| {
            x.base_type == "IO_SPECIAL"
        }).for_each(|x1| {
            println!("{}", x1);
        })
    }

    #[test]
    fn filter_instructions_02(){

    }
}