// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Display, Formatter};
use crate::instruction_decoder::bit_utils::set_bits;
use memory_mapping_unit::MappingUnit;


mod memory_mapping_unit;
pub mod io_device_emulator;
pub(crate) mod complex_instruction_executer;




//#[derive(Debug)]
pub(crate) struct ExecutionContext {
    pub(crate) ac:[u16;4], // Accumulators AC0..AC3
    pub(crate) br:[u16;2], // Base registers BR2,BR3

    pub(crate) sp:u16,  // Stack pointer
    pub(crate) fp:u16,  //Frame pointer
    pub(crate) sl:u16, //Stack Limit
    pub(crate) ip:u16, //instruction pointer

    pub(crate) zero_flag:bool, //ALU zero flag
    pub(crate) carry_flag:bool, //ALU carry flag
    pub(crate) overflow_flag:bool, //ALU overflow flag

    pub(crate) ion:bool, // Global interrupts enable flag
    pub(crate) ibn:bool, // Nested interrupts enable flag (The Interrupt Branch and Nest (IBN) flag determines whether interrupts will result in entry to a single master service routine or in automatic hardware vectoring to one of several individual service routines)


    pub(crate) mapping_unit: MappingUnit,
    pub(crate) mapsi_or_mapsd_active_for_one_instruction: bool, // used for controlling user mode for only one instruction
    pub(crate) mapsi_or_mapsd_active_for_one_instruction_target_ip: u16,
    pub(crate) interrupt_priority_mask:u16,
    // Following fields are just for debugging runtime in detail
    pub(crate) discovered_subroutines : HashMap<String,HashSet<u16>>,
    pub(crate) call_stack_debug_information : VecDeque<String>,
    pub(crate) rtc_initialized : bool,
}

impl Display for ExecutionContext{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AC0:{:x} AC1:{:x} AC2:{:x} AC3:{:x}, BR2:{:x} BR3:{:x}, FP:{:x} SP:{:x} SL:{:x}, ZF:{}, CF:{}, OVF:{}, user:{}, EM:{}", self.ac[0], self.ac[1], self.ac[2], self.ac[3],self.br[0],self.br[1],self.fp,self.sp,self.sl, self.zero_flag as u8, self.carry_flag as u8, self.overflow_flag as u8, self.mapping_unit.msr.get_current_user_info_str(), self.is_expanded_memory_for_the_current_user() as u8 )
    }
}

impl ExecutionContext{

    const TOP_OF_EXECUTIVE_STACK : u16 = 0x6aba;

    pub(crate) fn is_expanded_memory_for_the_current_user(&self) -> bool{
        if self.mapping_unit.msr.user_mode{
            self.mapping_unit.msr.user_expanded_memory
        }else { 
            self.mapping_unit.msr.executive_expanded_memory
        }
        
    }

    pub(crate) fn load_msr_word(&mut self, msr:u16) {
        self.mapping_unit.msr.set_msr_word(msr);
        println!("User : {} and SP : {:#x} after MSR load",self.mapping_unit.msr.get_current_user_info_str(),self.sp);
    }
    pub(crate) fn get_ac01_compound(&self) -> u32 {
        (self.ac[0] as u32) << 16 | self.ac[1] as u32
    }

    pub(crate) fn set_ac01_compound(&mut self, dw : u32) {
        self.ac[0] = (dw >> 16) as u16;
        self.ac[1] = (dw & 0xffff) as u16;
    }


    pub(crate) fn push_a_single_word_to_the_stack(&mut self, data_word: u16) {
        self.sp -=1;
        self.mapping_unit.write_word_to_memory(self.sp,data_word,true);

        debug_assert!( self.sp > self.sl);
    }

    pub(crate) fn pop_a_single_word_from_the_stack(&mut self) -> u16 {

        let data_word = self.mapping_unit.read_word_from_memory(self.sp,true);
        self.sp +=1;

        // TODO : Below check do not work on stack changes
        //debug_assert!( self.sp <= Self::TOP_OF_EXECUTIVE_STACK, "Stack underflow");
        data_word
    }

    pub(crate) fn dump_stack(&mut self)  {

        println!("Stack elements up to top of executive stack");
        for i in self.sp..=Self::TOP_OF_EXECUTIVE_STACK {
            let data_word = self.mapping_unit.read_word_from_memory(i, true);
            println!("{:#x} : {:#x}", i, data_word);
        }

    }

    pub(crate) fn encode_carry_and_overflow(&self) -> u16  {
        let mut carry_and_overflow = 0_u16;
        if self.carry_flag {
            carry_and_overflow |= 1;
        }
        if self.overflow_flag {
            carry_and_overflow |= 2;
        }

        carry_and_overflow

    }

    pub(crate) fn decode_carry_and_overflow(&mut self, compound : u16 )  {
        debug_assert!(compound < 4);

        self.carry_flag = compound & 1 == 1;
        self.overflow_flag = compound & 2 == 2;


    }

    pub(crate) fn push_status(&mut self) {
        let mut base_word  = 0x3f8_u16;

        set_bits(&mut base_word,0,0,self.ion as u16);
        set_bits(&mut base_word,1,1,self.ibn as u16);
        set_bits(&mut base_word,2,2,self.overflow_flag as u16);
        set_bits(&mut base_word,3,3,self.carry_flag as u16);

        let expanded_mem = if self.mapping_unit.msr.user_mode {self.mapping_unit.msr.user_expanded_memory} else { self.mapping_unit.msr.executive_expanded_memory };
        set_bits(&mut base_word,4,4,expanded_mem as u16);
        set_bits(&mut base_word,5,5, !self.mapping_unit.msr.user_mode as u16); //Executive mode is inverted user_mode

        self.push_a_single_word_to_the_stack(base_word);
    }

    pub(crate) fn generate_rtc_interrupt(&mut self) {

        writeln!(self.mapping_unit.log_writer.borrow_mut(),"Generating RTC interrupt");
        println!("Generating RTC interrupt");

        const QUICK_RTC_INTERRUPT:bool = false;

        if QUICK_RTC_INTERRUPT {
            let rtc_counter = self.mapping_unit.read_from_physical_memory_for_debug_or_test(0x327);
            if rtc_counter > 10 {
                println!("RTC hack to speed up sim, original timestamp : {}",rtc_counter);
                self.mapping_unit.write_to_physical_memory_for_debug_or_test(0x327,5);
            }
        }

        self.generate_interrupt(0o14);

    }

    pub fn generate_interrupt(&mut self, interrupting_device_id : u16) {

        let info_str =
        writeln!(self.mapping_unit.log_writer.borrow_mut(),"Generating interrupt for {:#o}", interrupting_device_id);
        println!("Generating interrupt for {:#o}", interrupting_device_id);

        let temp1_msr = self.mapping_unit.msr.get_msr_word();
        self.mapping_unit.msr.user_mode = false;
        self.mapping_unit.msr.executive_data_map = false;

        debug_assert!(self.ibn, "only IBN supported");
        let (isp, isl) = self.mapping_unit.get_interrupt_stack();
        let icnt = self.mapping_unit.get_icnt();

        let (isr_adr, ibn) = self.mapping_unit.get_interrupt_vector(interrupting_device_id);
        debug_assert!(ibn, "only IBN supported");
        debug_assert!(icnt == 0, "not nested interrupts in sim");

        let temp2_sp = self.sp;
        let temp3_sl = self.sl;

        self.sp = isp;
        self.sl = isl;

        self.mapping_unit.set_icnt(icnt + 1);

        self.push_a_single_word_to_the_stack(temp3_sl);
        self.push_a_single_word_to_the_stack(temp2_sp);
        self.push_a_single_word_to_the_stack(temp1_msr);
        self.push_a_single_word_to_the_stack(self.ip);

        self.push_a_single_word_to_the_stack(self.mapping_unit.get_cmask());

        self.ip = isr_adr; // jump to ISR
    }

    /** Similar to the interrupt routine*/
    pub(crate) fn call_trap(&mut self, trap_index:u8) {

        let temp1_msr = self.mapping_unit.msr.get_msr_word();

        self.mapping_unit.msr.executive_data_map = false;

        let temp2_sp = self.sp;
        let temp3_sl = self.sl;

        if self.mapping_unit.msr.user_mode{
            // if in user mode switch to executive stack
            let (xsp,xsl) = self.mapping_unit.get_executive_stack();
            self.sp = xsp;
            self.sl = xsl;
            self.mapping_unit.msr.user_mode = false;
        }

        self.push_a_single_word_to_the_stack(temp3_sl);
        self.push_a_single_word_to_the_stack(temp2_sp);
        self.push_a_single_word_to_the_stack(temp1_msr);
        self.push_a_single_word_to_the_stack(self.ip + 1);
        self.push_a_single_word_to_the_stack(self.mapping_unit.get_cmask());

        let trap_adr = self.mapping_unit.get_trap_vector(trap_index);
        self.ip = trap_adr; // jump to trap vector


    }

    pub(crate) fn store_byte_to_mem(&mut self, word_adr: u16, second_byte: bool, data_byte: u8) {
        let read_word = self.mapping_unit.read_word_from_memory(word_adr, true);

        let word_to_write;
        if !second_byte {
            word_to_write = (read_word & 0x00ff) | ((data_byte as u16) << 8);
        }else {
            word_to_write = (read_word & 0xff00) | data_byte as u16;
        }

        self.mapping_unit.write_word_to_memory(word_adr,word_to_write,true);
    }

    pub(crate) fn load_byte_from_mem(&mut self, word_adr: u16, second_byte: bool) -> u8 {
        let read_word = self.mapping_unit.read_word_from_memory(word_adr, true);

        let selected_byte : u8;
        if !second_byte {
            selected_byte = ((read_word & 0xff00) >> 8) as u8;
        }else {
            selected_byte = (read_word & 0xff) as u8;
        }

        selected_byte

    }

    pub(crate) fn read_word_from_mem_using_instruction_map_pc_relative(&mut self, offset:u16, mark_as_data:bool) -> u16{
        
        self.mapping_unit.read_word_from_mem_using_instruction_map(self.ip + offset,mark_as_data, self.mapping_unit.msr.is_executive())
     
    }

    pub(crate) fn load_initial_memory(&mut self, mem : Vec<u16>){

        self.mapping_unit.load_initial_memory(mem);

    }

    pub(crate) fn temporarily_activate_user_instruction_map_for_data_referencing(&mut self, target_ip : u16){
        self.mapping_unit.temporary_use_user_instruction_map_for_data_referencing = true;
        self.mapsi_or_mapsd_active_for_one_instruction = true;
        self.mapsi_or_mapsd_active_for_one_instruction_target_ip = target_ip;
    }

    pub(crate) fn temporarily_activate_user_data_map_for_data_referencing(&mut self, target_ip : u16){
        self.mapping_unit.temporary_use_user_data_map_for_data_referencing = true;
        self.mapsi_or_mapsd_active_for_one_instruction = true;
        self.mapsi_or_mapsd_active_for_one_instruction_target_ip = target_ip;
    }

    pub(crate) fn clear_temporary_used_user_maps(&mut self) {
        self.mapping_unit.temporary_use_user_instruction_map_for_data_referencing = false;
        self.mapping_unit.temporary_use_user_data_map_for_data_referencing = false;
        self.mapsi_or_mapsd_active_for_one_instruction = false;
    }
    
    pub(crate) fn new() -> ExecutionContext{
        
        //let mut mapping_unit = MappingUnit::new();
        //mapping_unit.log_writer = 

                
        ExecutionContext{
            ac: [0,0,0,0],
            br: [0,0],
            sp: 0,
            fp: 0,
            sl: 0,
            ip: 0,
            zero_flag: false,
            carry_flag: false,
            overflow_flag: false,
            ion : false,
            ibn : false,
            mapping_unit : MappingUnit::new(),
            mapsi_or_mapsd_active_for_one_instruction: false,
            mapsi_or_mapsd_active_for_one_instruction_target_ip: 0,
            interrupt_priority_mask: 0,
            discovered_subroutines: HashMap::<String,HashSet<u16>>::new(),
            call_stack_debug_information: Default::default(),
            rtc_initialized: false,
            
        }
    }
    
}