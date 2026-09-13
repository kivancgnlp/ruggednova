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

pub(crate) fn parse_ins64_pdf_parts_to_bin_file() -> Result<[u16; 65536], Error> {


    let mut mem = [0_u16; 65536];
    let mut prev_adr_word = 0_u16;

    let mut mem_wr_map = HashMap::<u16,u16>::new();

    let (mut loaded, mut gaps, mut unprinted) = (0_u32, 0_u32, 0_u32);
    let (mut malformed, mut duplicates, mut relocations) = (0_u32, 0_u32, 0_u32);
    let (lowest_adr, mut highest_adr) = (0_u16, 0_u16);
    let mut widest_gap = (0_u16, 0_u16, 0_u16);

        let file_str = std::fs::read_to_string("Data/Diagnostic images/1664 INSTRUCTION TEST (INS64)/ins64_addr_word.txt")?;

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

                if oct_adr != prev_adr_word {
                    // Gaps are normal: .BLK reserved storage and .TXT string bodies are not
                    // printed with an address, and .LOC jumps around. Count them rather than
                    // printing 35 lines, but keep the widest ones so an unexpected one shows.
                    if oct_adr > prev_adr_word {
                        gaps += 1;
                        let gap_width = oct_adr - prev_adr_word;
                        unprinted += gap_width as u32;
                        if gap_width > widest_gap.2 {
                            widest_gap = (prev_adr_word, oct_adr, gap_width);
                        }
                    } else {
                        // A backward step is a .LOC directive relocating the assembly, not a gap.
                        relocations += 1;
                    }
                    prev_adr_word = oct_adr;
                }
                prev_adr_word +=1;

                if oct_adr > highest_adr { highest_adr = oct_adr; }
                loaded += 1;

                if mem_wr_map.contains_key(&oct_adr){
                    duplicates += 1;
                    println!("Address {:#o} previously written by {}, current line : {}",oct_adr, mem_wr_map.get(&oct_adr).unwrap(),i);
                }else {
                    mem_wr_map.insert(oct_adr, oct_val);
                }
            }else {
                malformed += 1;
                println!("Error parsing line : {}", i);
            }

        }

    // A listing that silently loses words does not fail where the words are missing -- it fails
    // wherever the program first dereferences one of them, which can be thousands of words away.
    // Two cases already cost a debugging session each: a `.BLK 0` operand read as an object word
    // clobbered the JMP ending LOAD4, and a two-page extraction shortfall left the C?TTY vector
    // zero, sending the error reporter to address 0. So say plainly what was loaded.
    println!("INS64 listing: {} words loaded, {:#o}..{:#o}, {} gaps ({} words not printed), {} .LOC relocations",
             loaded, lowest_adr, highest_adr, gaps, unprinted, relocations);
    println!("  widest gap {:#o} -> {:#o} ({} words) -- expect .BLK storage or a .TXT body",
             widest_gap.0, widest_gap.1, widest_gap.2);

    if malformed > 0 || duplicates > 0 {
        println!("  !! {} malformed line(s), {} duplicate address(es) -- the image is INCOMPLETE",
                 malformed, duplicates);
    }

    Ok(mem)
}

pub(crate) fn parse_ins02_pdf_parts_to_bin_file() -> Result<[u16; 65536], Error> {
    let parts = ["(0 - 232)","A", "B", "C", "D","E","F","G","H","I","J","K"];

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
                    println!("Address {:#o} previously written by {}, current line : {}, current file : {}",oct_adr, mem_wr_map.get(&oct_adr).unwrap(),i,part_char);
                }else {
                    mem_wr_map.insert(oct_adr, oct_val);
                }
            }

        }
    }
    Ok(mem)
}