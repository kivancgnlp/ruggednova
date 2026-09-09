// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Error, Read, Write};


pub(crate) fn load_binary_file(input_file_path_str: &str) -> Result<Vec<u16>, Error>{

    let mut f = File::open(input_file_path_str)?;
    let mut read_buffer:Vec<u16> = Vec::new();

    let mut br = BufReader::new(f);

    let mut be_bytes = [0_u8; 2];

    let mut read_result = br.read_exact(&mut be_bytes);

    while read_result.is_ok() {
        read_buffer.push(u16::from_be_bytes(be_bytes));
        read_result = br.read_exact(&mut be_bytes);

    }

    Ok(read_buffer)
}

pub(crate) fn dump_memory_to_file(file_name: &str, mem : &[u16;65536]) -> Result<(), Error>{

    let f = File::create(file_name)?;
    let mut bw = BufWriter::new(f);

    mem.iter().for_each(|&word| {
        bw.write_all(&word.to_be_bytes()).unwrap();
    });


    Ok(())
}


pub(crate) fn parse_pdf_parts_to_bin_file() -> Result<[u16; 65536], Error> {
    let parts = ["(0 - 232)","A", "B", "C", "D","E","F","G","H"];

    let mut mem = [0_u16; 65536];

    let mut mem_wr_map = HashMap::<u16,u16>::new();

    for part_char in parts {
        let file_name = format!("Data/Diagnostic images/1602 INSTRUCTION TEST (INS02)/Part {}.txt", part_char);

        let file_str = std::fs::read_to_string(file_name)?;

        for i in file_str.lines() {

            //println!("{}", i);
            let oct_val = i.split(" ").nth(1).and_then(|oct_str| {
                u16::from_str_radix(oct_str, 8).ok()
            });

            let oct_adr = i.split(" ").nth(0).and_then(|oct_str| {
                u16::from_str_radix(oct_str, 8).ok()
            });


            if let (Some(oct_adr),Some(oct_val))  = (oct_adr,oct_val) {
                mem[oct_adr as usize] = oct_val;
                if mem_wr_map.contains_key(&oct_adr){
                    println!("Address previously written {}", mem_wr_map.get(&oct_adr).unwrap());
                }else {
                    mem_wr_map.insert(oct_adr, oct_val);
                }
            }

        }
    }
    Ok(mem)
}