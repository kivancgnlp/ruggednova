// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::cell::RefCell;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Result, Write};
use std::path::PathBuf;
use std::rc::Rc;

use loaders::paper_tape_read_ab_file_reader::paper_tape_image_loader;

mod instruction_identifier;
mod instruction_decoder;
mod assembler;

mod loaders;

pub(crate) mod virtual_machine;


fn main() -> std::io::Result<()> {



    let input_file_path = PathBuf::from("Data/Diagnostic images/095-000005-01__Nova_Logic_Test__1969.ab");
    //let input_file_path = PathBuf::from("Data/Diagnostic images/novacputest.ab");


    let mut ex = virtual_machine::ExecutionContext::new();

    //let boot_loader_memory = load_program_to_memory(&input_file_path)?;
    let boot_loader_memory = paper_tape_image_loader(input_file_path.to_str().unwrap())?;
    

    ex.load_initial_memory(Vec::from(boot_loader_memory));


    let linear_disassembler_mode = true;
    let instruction_limit = 5_000;
    let generate_trace_disassembly = true;
    ex.ip = 0x40;


    let file_name_stem = input_file_path.file_stem().unwrap().to_str().unwrap().to_string();

    let mut output_file_name = PathBuf::from("Data/");
    output_file_name.push(file_name_stem + if linear_disassembler_mode {"_linear.txt"} else {"_trace.txt"});


    //let output_stream : Rc<RefCell<Box<dyn Write>>> = Rc::new(RefCell::new(Box::new(stdout())));
    let output_stream : Rc<RefCell<Box<dyn Write>>> = Rc::new(RefCell::new(Box::new(BufWriter::new(File::create(&output_file_name)?))));


    ex.mapping_unit.set_log_writer(output_stream.clone());
    let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context(ex, output_stream.clone(), linear_disassembler_mode,generate_trace_disassembly)?;

    if let Err(e) = instruction_decoder.decode_instructions(instruction_limit){
        eprintln!("Error while decoding instructions {:?}", e);
    };

    instruction_decoder.dump_stats();
    

    Ok(())


   
}



fn load_program_to_memory(input_file_path : &PathBuf) -> Result<Vec<u16>> {

    let mut input_stream = BufReader::new(File::open(&input_file_path)?);
    let file_size = input_file_path.metadata()?.len();
    let mut word_buffer = Vec::<u16>::with_capacity((file_size / 2) as usize);

    let mut dw_buffer = [0u8;2];
    
    let mut read_result =input_stream.read_exact(&mut dw_buffer);

    loop {
        match read_result{
            Ok(_) => {
                let word = u16::from_be_bytes(dw_buffer);
                word_buffer.push(word);
                read_result=input_stream.read_exact(&mut dw_buffer);
            } Err(_)=>{
                println!("End of file reached.");
                break;
            }
        }
    }


    Ok(word_buffer)

}



