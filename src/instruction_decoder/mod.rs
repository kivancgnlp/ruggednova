// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::cell::RefCell;
use std::io::{stdout, Error, Write};
pub(crate) mod alc_format_instruction_decoder;
mod memory_reference_wo_acc_format_instruction_decoder;
mod io_format_instruction_decoder;
pub(crate) mod bit_utils;
mod memory_reference_with_acc_format_instruction_decoder;
pub(crate) mod extended_memory_acc_format_instruction_decoder;
pub(crate) mod generic_instruction_format_decoder;
pub(crate) mod alc_format_data_fields;
mod alc_format_instruction_executor;
mod memory_reference_format_data_fields;
mod io_format_data_fields;

use std::io::Result;
use std::rc::Rc;
use crate::virtual_machine::ExecutionContext;
use crate::instruction_identifier::instruction_data_parser::InstructionData;

use crate::instruction_identifier::InstructionIdentifier;

const ACCUMULATOR_NAMES: [&str; 4] = ["AC0","AC1","AC2","AC3"];


pub(crate) struct InstructionDecoder{
    ec: ExecutionContext,
    instructions_info: InstructionIdentifier,
    //disassembly_output_file: BufWriter<File>,
    disassembly_output_file: Rc<RefCell<Box<dyn Write>>>,
    linear_disassembler_mode : bool,
    generate_trace_disassembly : bool,
}


impl InstructionDecoder {

    pub fn new_with_execution_context(ec : ExecutionContext, disassembly_output_file : Rc<RefCell<Box<dyn Write>>>, linear_disassembler_mode : bool, generate_trace_disassembly: bool) -> Result<InstructionDecoder> {
        Ok(InstructionDecoder{ec,instructions_info: InstructionIdentifier::new()?,disassembly_output_file,linear_disassembler_mode,generate_trace_disassembly })
    }

    pub fn new_with_execution_context_default_for_tests(ec : ExecutionContext) -> Result<InstructionDecoder> {
        Ok(InstructionDecoder{ec,instructions_info: InstructionIdentifier::new()?,disassembly_output_file:Rc::new(RefCell::new(Box::new(stdout()))), linear_disassembler_mode : false,generate_trace_disassembly :true })
    }

    pub(super) fn decode_and_execute_instruction(&mut self, instruction: &InstructionData, instruction_word : u16, extra_instruction_words: &Vec<u16>) -> Result<()> {


        let ip_before_ins = self.ec.ip;
        let ins_str = self.decode_instruction(instruction, instruction_word, extra_instruction_words);

        
        let mut asm_line = format!("U:{} IP:{:04x} [{:04x}] {}", self.ec.mapping_unit.msr.get_current_user_info_str(), ip_before_ins, instruction_word, ins_str);


        if !self.linear_disassembler_mode {
            asm_line.push_str(format!(" ({})", self.ec).as_str());
        }

        if self.generate_trace_disassembly || self.linear_disassembler_mode{
            let noktalar = ".".repeat(self.ec.call_stack_debug_information.len());
            write!(self.disassembly_output_file.borrow_mut(),"{}",noktalar)?;
            writeln!(self.disassembly_output_file.borrow_mut(), "{}", asm_line)?;
        }

        

        Ok(())

    }

    fn decode_instruction(&mut self, instruction: &InstructionData, instruction_word: u16, extra_instruction_words: &Vec<u16>) -> String {
        let decoded;
        let initial_ip = self.ec.ip;

        let opt_ecs = if self.linear_disassembler_mode {None} else {Some(&mut self.ec)};

        if instruction.base_type == "ALC" {
            decoded = alc_format_instruction_decoder::decode(instruction_word,opt_ecs);
        } else if instruction.base_type == "MEM_REF_WO_ACC" {
            decoded = memory_reference_wo_acc_format_instruction_decoder::decode(instruction_word, initial_ip, opt_ecs);
        } else if instruction.base_type == "MEM_REF_WITH_ACC" {
            decoded = memory_reference_with_acc_format_instruction_decoder::decode(instruction_word, initial_ip, opt_ecs);
        } else if instruction.base_type == "IO" {
            decoded = io_format_instruction_decoder::decode(instruction_word,opt_ecs);
        }else if extended_memory_acc_format_instruction_decoder::EXTENDED_MEMORY_TO_ACC_INS.contains(&instruction.mnemonic.as_str()) ||
            extended_memory_acc_format_instruction_decoder::EXTENDED_ACC_TO_MEMORY_INS.contains(&instruction.mnemonic.as_str()) {
            decoded = extended_memory_acc_format_instruction_decoder::decode(instruction_word, extra_instruction_words[0], opt_ecs);
        }else if let Some(d) = generic_instruction_format_decoder::decode(instruction.mnemonic.as_str(), instruction_word, extra_instruction_words.get(0),opt_ecs) {
            decoded = d;
        } else { // Last resort for displaying instruction
            if let Some(ew1) = extra_instruction_words.get(0) { // format it using hex if any extra word
                decoded = format!("{} {:#x}", instruction.mnemonic, ew1);
            }else{
                decoded = format!("{}", instruction.mnemonic);
            }

        }

        let usual_next_ip = initial_ip + 1 + extra_instruction_words.len() as u16;
        if self.linear_disassembler_mode {
            self.ec.ip = usual_next_ip;
        }else{

            if self.ec.ip == initial_ip {
                eprintln!("IP stayed same after instruction execution. Possible buggy instruction {}",instruction.mnemonic);
                //self.ec.ip = usual_next_ip;
            }
        }

        decoded
    }

    pub(crate) fn decode_instructions(&mut self, instruction_limit: u32) -> Result<()>{
        let mut instruction_counter = 0_u32;
        let mut ip_before_instruction_execution;

        loop {

            self.ec.mapping_unit.use_instruction_map(false); // for all instructions data acceses are using data map except for instructions that uses PCRelative adressing
            let instruction_word = self.ec.read_word_from_mem_using_instruction_map_pc_relative(0,false);

            ip_before_instruction_execution = self.ec.ip;
            /*
            if self.ec.ip == 0x8479{
                println!("IP breakpoint");
            }

             */

            let instruction_resolve_result = self.instructions_info.identify_instruction(instruction_word);

            if(instruction_resolve_result.is_none() && self.linear_disassembler_mode){
                writeln!(self.ec.mapping_unit.log_writer.borrow_mut(),"Unidentified instruction at IP : {:#x} : [{:#x}]",self.ec.ip,instruction_word);
                self.ec.ip += 1;
                continue; // ignore in linear disassembler mode
            }

            if(instruction_resolve_result.is_none() && self.linear_disassembler_mode == false){
                return Err(Error::other("Unidentified instruction"));
            }

            let instruction = instruction_resolve_result.unwrap();

            let mut extra_words = Vec::new();

            for i in 0..instruction.following_word_count{
                extra_words.push(self.ec.read_word_from_mem_using_instruction_map_pc_relative((1 + i) as u16,false)); // Burada mark_as_data dememin nedeni bazen instruction immediate operandları kullanılıyor. Yani bura esasında data olabilir gibi işaretlenmeli
            }


            self.decode_and_execute_instruction(&instruction, instruction_word, &extra_words)?;

            if self.ec.mapsi_or_mapsd_active_for_one_instruction && ip_before_instruction_execution == self.ec.mapsi_or_mapsd_active_for_one_instruction_target_ip{
                self.ec.clear_temporary_used_user_maps();
            }
            
            if self.ec.ion && self.ec.rtc_initialized && instruction_counter % 5000 == 0 && self.ec.mapping_unit.get_icnt() == 0{
                self.ec.generate_rtc_interrupt();
            }

           

            if self.ec.ion && self.ec.mapping_unit.get_icnt() == 0{
                
                // This is the place for generating interrupts if pending IO interrupts
             

                

            }


            instruction_counter += 1;            
            if self.generate_trace_disassembly && instruction_counter >= instruction_limit{
                break
            }

            if self.ec.cpu_halted {
                println!("CPU halted");
                break
            }
        }

        Ok(())
    }

    pub(crate) fn dump_stats(&self) {
        //self.instructions_info.dump_stats();

        for (mnm, addresses) in &self.ec.discovered_subroutines{
            print!("Discovered {} subroutines : ", mnm);
            addresses.iter().for_each(|i| print!("{:#x} ", i));
            println!();
        }
    }

    pub(crate) fn peek_ec(&self) -> &ExecutionContext{
        &self.ec
    }
}



#[cfg(test)]
mod tests {
    use crate::assembler::Assembler;
    use crate::instruction_decoder;
    use super::*;

    #[test]
    fn mapsi_user_map_test() -> Result<()> {

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.trace_phy_mem_accesses = true;

        let asm = Assembler::new()?;

        let test_asm_lines = "\
        STEM\n\
        LDFNW AC0 0x2 ; Access word\n\
        LDFNW AC1 0x1 ; Control word\n\
        WRWRD\n\
        LDFNW AC2 0x2 ; MSR word\n\
        WMSR AC2\n\
        MAPSI\n\
        STTNA AC1 0x0\n\
        LDFNA AC2 0X400\n\
        SUB # AC1,AC2 SZR
        HALT\n";

        let abc  =asm.assemble_lines(test_asm_lines);

        ec.load_initial_memory(asm.assemble_lines(test_asm_lines));
        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ec)?;
        instruction_decoder.decode_instructions(10)?;
        assert_eq!(instruction_decoder.peek_ec().ip, 0x10);

        Ok(())
    }
    #[test]
    fn executive_data_map_test() -> Result<()> {

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.trace_phy_mem_accesses = true;

        let asm = Assembler::new()?;

        let test_asm_lines = "\
        STEM\n\
        LDFNW AC0 0x1 ; Access word\n\
        LDFNW AC1 0x1 ; Control word\n\
        WRWRD\n\
        EXMAP\n\
        STTNA AC1 0x0\n\
        DXMAP\n\
        LDFNA AC2 0X400\n\
        SUB # AC1,AC2 SZR
        HALT\n";

        let abc  =asm.assemble_lines(test_asm_lines);

        ec.load_initial_memory(asm.assemble_lines(test_asm_lines));
        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ec)?;
        instruction_decoder.decode_instructions(9)?;
        assert_eq!(instruction_decoder.peek_ec().ip, 0x0e);

        Ok(())
    }

    #[test]
    fn rtfni_map_test() -> Result<()> {

        let mut ec = ExecutionContext::new();

        let asm = Assembler::new()?;

        ec.ip = 7;
        ec.sp = 0x100;
        ec.fp = 0x100;
        ec.mapping_unit.trace_phy_mem_accesses = true;

        let test_asm_lines = "\
        DW 0x0\n\
        DW 0x10\n\
        DUPW 0x5\n\
        STEM\n\
        LDFNW AC0 0x2\n\
        LDFNW AC1 0x1\n\
        WRWRD\n\
        LDFNW AC0 0x50 ; SL\nPSH AC0\n\
        LDFNW AC0 0x50 ; SP\nPSH AC0\n\
        LDFNW AC0 0xF7FA ; MSR\nPSH AC0\n\
        LDFNW AC0 0x0 ; Return adr (IP)\nPSH AC0\n\
        LDFNW AC0 0x20 ; CMASK\nPSH AC0\n\
        RTFNI\n\
        DUPW 0x3E3\n\
        LDFNW AC2 0x1234\n\
        JMP +0\n";

        let abc  =asm.assemble_lines(test_asm_lines);

        ec.load_initial_memory(asm.assemble_lines(test_asm_lines));
        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ec)?;
        instruction_decoder.decode_instructions(20)?;
        assert_eq!(instruction_decoder.peek_ec().ac[2], 0x1234);
        println!("{}",instruction_decoder.peek_ec());

        Ok(())
    }
    #[test]
    fn test_non_existent_ins() -> Result<()> {

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.load_initial_memory(vec![0x6700,0x8028]);
        let mut instruction_decoder = InstructionDecoder::new_with_execution_context_default_for_tests(ec)?;

        instruction_decoder.decode_instructions(4)?;

        Ok(())
    }

    #[test]
    fn machine_code_decode_scratchpad() -> Result<()> {

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.load_initial_memory(vec![0x8068,3]);
        let mut instruction_decoder = InstructionDecoder::new_with_execution_context_default_for_tests(ec)?;

        instruction_decoder.decode_instructions(4)?;

        Ok(())
    }


}