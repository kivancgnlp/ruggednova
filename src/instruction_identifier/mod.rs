// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::collections::HashMap;
use instruction_data_parser::InstructionData;

pub mod instruction_data_parser;

/*
static INS_DEC_WARNING_REPORT: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();

fn get_report_file() -> &'static Mutex<BufWriter<File>> {
    INS_DEC_WARNING_REPORT.get_or_init(|| {
        let file = File::create("Instruction_decode_warnings.txt").unwrap();
        Mutex::new(BufWriter::new(file))
    })
}
 */

pub(crate) struct InstructionIdentifier {
    instruction_infos : Vec<InstructionData>,
    instruction_info_cache: HashMap<u16,InstructionData>,
    instruction_info_cache_utilization : (usize, usize),
    mnemonic_stats: HashMap<String,usize>,
    instruction_class_stats: HashMap<String,usize>,
    may_conflicting_instructions_log: HashMap<u16,String>,

}

impl InstructionIdentifier {

    pub(crate) fn new() -> Result<InstructionIdentifier, std::io::Error> {

        let instructions = instruction_data_parser::parse_instruction_informations()?;
        Ok(InstructionIdentifier {instruction_infos:instructions,
            instruction_info_cache:HashMap::new(),
            instruction_info_cache_utilization : (0,0),
            mnemonic_stats:HashMap::new(),
            instruction_class_stats:HashMap::new(),
            may_conflicting_instructions_log: HashMap::new() })
    }

    fn display_sorted_histogram(title : &str,histogram: &HashMap<String,usize>) {
        println!("------ {} ---------",title);

        let mut map_entries: Vec<_> = histogram.iter().collect();

        map_entries.sort_by(|a, b| b.1.cmp(&a.1));

        for (key,value) in map_entries{
            println!("{}\t{}",key,value);
        }

    }

    fn display_confused_instructions(map: &HashMap<u16,String>) {


        let mut map_entries: Vec<_> = map.iter().collect();

        map_entries.sort_by(|a, b| b.0.cmp(&a.0));

        for (key,value) in map_entries{
            if value != "NP"{
                println!("{:#x}\t{}",key,value);
            }

        }

    }
    pub(crate) fn dump_stats(&self) {

        Self::display_confused_instructions(&self.may_conflicting_instructions_log);
        Self::display_sorted_histogram("Mnemonic Occurrences", &self.mnemonic_stats);
        Self::display_sorted_histogram("By Instruction Class", &self.instruction_class_stats);

    }

    pub(crate) fn get_instructions(&self) -> &Vec<InstructionData> {

        &self.instruction_infos

    }

    fn update_stats(&mut self,ins : &InstructionData) {

        self.mnemonic_stats.entry(ins.mnemonic.clone()).and_modify(|v| { *v += 1 })
            .or_insert(1);

        self.instruction_class_stats.entry(ins.base_type.clone()).and_modify(|v| { *v += 1 }).or_insert(1);

    }

    fn log_multi_match_case(&mut self, instruction_word: u16, matched_instructions: &Vec<InstructionData>, selected_candidate: &InstructionData) {
        if self.may_conflicting_instructions_log.contains_key(&instruction_word){
            return; // return if already logged
        }


        if instruction_word & 0x800f == 0x8008{ // Eliminate not possible ALC instructions
            let alc_filtered_out = matched_instructions.iter().filter(|x| {
                x.base_type != "ALC"
            }).collect::<Vec<_>>();

            if alc_filtered_out.len() < 2 {
                self.may_conflicting_instructions_log.insert(instruction_word,"NP".to_string());
                return;
            }

        }

        let mut matched_instructions_str = String::new();

        for instruction in matched_instructions.iter() {
            let line = format!("{}\n", instruction);
            matched_instructions_str.push_str(&line);

            if instruction.mnemonic != selected_candidate.mnemonic{ // Selected instruction bit mask count sanity check
                let bit_count_1 = u16::count_ones(instruction.match_mask);
                let bit_count_2 = u16::count_ones(selected_candidate.match_mask);

                if bit_count_1 == bit_count_2 {
                    let warning_str = format!("Candidate instructions and selected instructions bit mask is similar");
                    eprintln!("{}", warning_str);
                    matched_instructions_str.push_str(&line);

                }
            }
        }

        self.may_conflicting_instructions_log.insert(instruction_word, matched_instructions_str);


    }


    pub(crate) fn identify_instruction(&mut self, instr_word : u16) -> Option<InstructionData> {

        let cache_lookup = self.instruction_info_cache.get(&instr_word);

        let matched_instruction_list = match cache_lookup {
            Some(lookup) => {
                self.instruction_info_cache_utilization.1 += 1;
                vec![lookup.clone()]
            }
            None => {
                self.instruction_info_cache_utilization.0 += 1;
                self.search_matching_instructions(instr_word)
            }
        };


        if matched_instruction_list.len() == 0{
            println!("No matched Instruction");
        }

        if matched_instruction_list.len() > 1{


            let selected_instruction = matched_instruction_list.iter().max_by(|a, b| {
                let a_bit_count = a.match_mask.count_ones();
                let b_bit_count = b.match_mask.count_ones();
                a_bit_count.cmp(&b_bit_count)
            });

            let selected_inst = selected_instruction.expect("No instruction found");

            self.instruction_info_cache.insert(instr_word, selected_inst.clone()); // add to cache for later lookups

            self.log_multi_match_case(instr_word, &matched_instruction_list, selected_inst);

            self.update_stats(selected_inst);
            return Some(selected_inst.clone());
        }


        matched_instruction_list.get(0).cloned()
    }

    fn search_matching_instructions(&mut self, instr_word: u16) -> Vec<InstructionData> {
        let mut masked_value;

        let mut matched_instruction_list = vec![];

        for instruction in self.instruction_infos.iter() {
            masked_value = instr_word & instruction.match_mask;
            if masked_value == instruction.match_value {
                matched_instruction_list.push(instruction.clone());
            }
        }
        matched_instruction_list
    }
}




#[cfg(test)]
mod tests {
    use std::collections::{HashMap};
    use crate::instruction_identifier::{InstructionIdentifier};

    use std::io::Result;
    use std::net::UdpSocket;
    use crate::instruction_decoder::bit_utils;

    #[test]
    fn test_01() -> Result<()> {

        let mut instruction_identifer = InstructionIdentifier::new()?;
        let result = instruction_identifer.identify_instruction(0x0974);

        assert_eq!(result.unwrap().mnemonic, "JSR");
        Ok(())
    }


    #[test]
    fn print_all_instructions_in_octal() -> Result<()> {

        // Bu test dokuman ile cross check icin kullanildi
        let mut instruction_identifer = InstructionIdentifier::new()?;

        instruction_identifer.instruction_infos.sort_by(|x, x1| {
            x.mnemonic.cmp(&x1.mnemonic)
        });
        for ins in instruction_identifer.instruction_infos{
            println!("{:6}\t{:#8o}\t{:#x}", ins.mnemonic, ins.match_value, ins.match_mask);
        }

        Ok(())
    }

    #[test]
    fn filter_instructions_scratchpad() -> Result<()> {

        // Bu test dokuman ile cross check icin kullanildi
        let instruction_identifer = InstructionIdentifier::new()?;

        instruction_identifer.instruction_infos.iter().filter(|ins| ins.following_word_count > 0).for_each(|ins| {
            println!("{:6}\t{:#8o}\t{}", ins.mnemonic,ins.match_value, ins.following_word_count);
        });

        Ok(())
    }

    #[test]
    fn duplicate_match_instruction_sanity_test() -> Result<()> {

        #[derive(Eq, Hash, PartialEq, Debug)]
        struct MatchPair{
            match_value : u16,
            match_mask : u16,
        }

        let instruction_identifer = InstructionIdentifier::new()?;

        let mut pair_set = HashMap::<MatchPair,String>::new();

        for ins in instruction_identifer.instruction_infos{
            let mp = MatchPair{match_value:ins.match_value,match_mask:ins.match_mask};

            if pair_set.contains_key(&mp){
                println!("Match already match. Previous vs current mnemonic {} {}",pair_set[&mp], ins.mnemonic);
                assert!(false);
            }else {
                pair_set.insert(mp,ins.mnemonic);
            }

        }

        println!("Number of unique pairs : {}", pair_set.len());
        Ok(())
    }

    #[test]
    fn invalid_match_pair_sanity_test() -> Result<()> {

        let instruction_identifer = InstructionIdentifier::new()?;

        for ins in instruction_identifer.instruction_infos{
            assert_eq!(ins.match_value & ins.match_mask, ins.match_value);
        }

        Ok(())
    }

    #[test]
    fn search_instruction_scratch_pad() -> Result<()> {

        let mut instruction_identifer = InstructionIdentifier::new()?;

        let result = instruction_identifer.identify_instruction(0x9118);

        result.inspect(|x| println!("{:?}", x));

        let conf = instruction_identifer.may_conflicting_instructions_log;
        println!("{:#?}", conf);
        Ok(())
    }

    #[test]
    fn bit_byte_instructions_test() -> Result<()> {
        
        let mut instruction_identifier = InstructionIdentifier::new()?;
        let mut ins_word: u16 = 0o114510; // SZBO instruction

        bit_utils::set_bits(&mut ins_word, 1, 2, 3); // set ABN = 3

        let result = instruction_identifier.identify_instruction(ins_word);

        assert_eq!(result.unwrap().mnemonic, "SZBO");
        Ok(())
    }

    #[test]
    #[ignore]
    fn instruction_identify_server() -> Result<()> {

        let mut instruction_identifier = InstructionIdentifier::new()?;
        let mut ins_word: u16 = 0; //
        let mut ins_bytes = [0_u8;2];

        let incoming_instruction_query_socket = UdpSocket::bind("127.0.0.1:5003").expect("Error while creating query socket");
        let outgoing_instruction_query_response_socket = UdpSocket::bind("127.0.0.1:0").expect("Error while creating query response socket");

        loop {
            let (received_byte_count,from_adr) = incoming_instruction_query_socket.recv_from(&mut ins_bytes).expect("Error while receiving incoming query");



            ins_word = u16::from_be_bytes(ins_bytes);

            println!("Received byte count : {}, from address : {:?}, word : {:#x} ", received_byte_count, from_adr, ins_word);

            if ins_word == 0xffff{
                println!("Quit signalled");
                break;
            }
            let result = instruction_identifier.identify_instruction(ins_word);



            match result {
                Some(ins_word) => {
                    println!("Replying : {}", ins_word.mnemonic.as_str());
                    outgoing_instruction_query_response_socket.send_to(ins_word.mnemonic.as_ref(), from_adr)?;
                }
                None => {
                    println!("Replying : ?");
                    outgoing_instruction_query_response_socket.send_to("?".as_ref(), from_adr)?;
                }
            } 
        }
      
        Ok(())
    }

}
