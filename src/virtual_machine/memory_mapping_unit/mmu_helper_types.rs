use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::io::{Stdout, Write};
use std::rc::Rc;
use crate::instruction_decoder::bit_utils::{get_bits, set_bits};
use crate::virtual_machine::memory_mapping_unit::MappingUnit;

pub(crate) struct MemoryStatusRegister {
    pub(crate) user_mode : bool, // (!XMD) (When power is first turned on, or after a panel reset, the RMU and processor are in Executive Mode)
    pub(crate) executive_expanded_memory : bool, // (XEM)
    pub(crate) user_expanded_memory : bool, // (UEM)
    pub(crate) executive_data_map : bool, // (XDM) (When enabled the executive data map allows the executive to map certain data memory references to any of physical memory. This allows the executive to have all instruction accesses and program-counter-relative data accesses mapped to the first 64k of physical memory, and to have all other data references mapped through an independent map to any other 64k words of physical memory in 1k-word pages)
    pub(super)user_data_map : bool, // (UDM) (When the user data map is enabled, all instruction fetches or program counter relative data accesses are mapped through Map 2 (for user 2), all other data accesses will be mapped using Map 3 (for user 2), these distinct data/instruction maps are only valid for users (2,4,6).)
    pub(super) dma_map : bool,
    pub(super) user_page_protection: bool, // User read/write/execute page protection
    pub(super) defer_indirect_protection: bool, // Defer (indirect) protection
    pub(super) io_protection: bool, // IO protection
    pub(super) dma_protection: bool, // DMA protection
    pub(crate) user : u8, // User or last active user (2-7)

    pub(crate) log_writer : Rc<RefCell<Box<dyn Write>>>,
}

pub(crate) struct MemoryViolationsRegister {
    pub(crate) dma_protection_error : bool,
    pub(crate) execute_protection_error : bool,
    pub(crate) read_protection_error : bool,
    pub(crate) write_protection_error : bool,
    pub(crate) defer_protection_error : bool,
    pub(crate) io_protection_error : bool,
    pub(crate) privileged_instruction_protection_error : bool,
    pub(crate) violation_occurred_during_single_cycle_operation : bool,

}

impl MemoryViolationsRegister {
    
    pub(crate) fn new() -> Self{
        MemoryViolationsRegister{
            dma_protection_error: false,
            execute_protection_error: false,
            read_protection_error: false,
            write_protection_error: false,
            defer_protection_error: false,
            io_protection_error: false,
            privileged_instruction_protection_error: false,
            violation_occurred_during_single_cycle_operation: false,
        }
    }
    
    pub(crate) fn clear(&mut self){
        self.dma_protection_error = false;
        self.execute_protection_error = false;
        self.read_protection_error = false;
        self.write_protection_error = false;
        self.defer_protection_error = false;
        self.io_protection_error = false;
        self.privileged_instruction_protection_error = false;
        self.violation_occurred_during_single_cycle_operation = false;
        
    }
    
    pub(crate) fn get_mvr_word(&self, user:u8) -> u16{
        let mut word = user as u16;

        set_bits(&mut word,0,0,self.dma_protection_error as u16);
        set_bits(&mut word,1,1,self.execute_protection_error as u16);
        set_bits(&mut word,2,2,self.read_protection_error as u16);
        set_bits(&mut word,3,3,self.write_protection_error as u16);
        set_bits(&mut word,4,4,self.defer_protection_error as u16);
        set_bits(&mut word,5,5,self.io_protection_error as u16);
        set_bits(&mut word,6,6,self.privileged_instruction_protection_error as u16);
        set_bits(&mut word,7,7,self.violation_occurred_during_single_cycle_operation as u16);
        
        word
    
    }

}

impl Display for MemoryStatusRegister {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"User mode:{}, executive_expanded_memory:{}, user_expanded_memory:{}, executive_data_map:{}, user_data_map:{}, dma_map:{}, user_page_protection:{}, defer_indirect_protection:{}, io_protection:{}, dma_protection:{}, user or last active user : {}",
               self.user_mode as u8,
               self.executive_expanded_memory as u8,
               self.user_expanded_memory as u8,
               self.executive_data_map as u8,
               self.user_data_map as u8,
               self.dma_map as u8,
               self.user_page_protection as u8,
               self.defer_indirect_protection as u8,
               self.io_protection as u8,
               self.dma_protection as u8,
               self.user)
    }
}

impl MemoryStatusRegister{

    pub(crate) fn new_with_logger(rc: Rc<RefCell<Box<dyn Write>>>) -> Self {
        MemoryStatusRegister{
            user_mode: false,
            executive_expanded_memory: false,
            user_expanded_memory: false,
            executive_data_map: false,
            user_data_map: false,
            dma_map: false,
            user_page_protection: false,
            defer_indirect_protection: false,
            io_protection: false,
            dma_protection: false,
            user: 0,
            log_writer: rc
        }
        
    }
    pub(crate) fn is_executive(&self) -> bool{
        !self.user_mode
    }
    pub(crate) fn get_current_user_info_str(&self) -> char {

        let info;

        if self.user_mode == false {
            info ='E';
        }else {
            info = match self.user {
                0 => 'D', // DMA shouldn't be displayed like that
                2 => '2',
                3 => '3',
                4 => '4',
                5 => '5',
                6 => '6',
                7 => '7',
                _ => unreachable!()
            }
        }

        info

    }
    pub(crate) fn load_msr_via_wmsr_instruction(&mut self, word: u16) {
        // Loads the Map Status Register from the specified accumulator. Bits 0, 1, and 3 of the
        // MSR are unaffected by this instruction

        //println!("Loading msr via WMSR instruction, before load : {}",self);

        self.user = get_bits(word, 13, 15) as u8;
        self.dma_protection = get_bits(word,9,9) == 1;
        self.io_protection = get_bits(word,8,8) == 1;
        self.defer_indirect_protection = get_bits(word,7,7) == 1;
        self.user_page_protection = get_bits(word,6,6) == 1;
        self.dma_map = get_bits(word,5,5) == 1;
        self.user_data_map = get_bits(word,4,4) == 1;


        // Bits 0, 1, and 3 not affected in this instruction
        self.user_expanded_memory = get_bits(word,2,2) == 1;

        if self.user_expanded_memory == false {
            eprintln!("Non expanded memory is not supported. During WMSR instruction");
        }

        if self.user_mode == true && self.user < 2{
            eprintln!("Bug 2 on msr load ?");
        }

        writeln!(self.log_writer.borrow_mut(),"Loading msr ({:#x})via WMSR instruction : {}",word,self);

    }

    pub(crate) fn set_msr_word(&mut self, word: u16) {

        self.user = get_bits(word, 13, 15) as u8;
        self.dma_protection = get_bits(word,9,9) == 1;
        self.io_protection = get_bits(word,8,8) == 1;
        self.defer_indirect_protection = get_bits(word,7,7) == 1;
        self.user_page_protection = get_bits(word,6,6) == 1;
        self.dma_map = get_bits(word,5,5) == 1;
        self.user_data_map = get_bits(word,4,4) == 1;
        self.executive_data_map = get_bits(word,3,3) == 1;
        self.user_expanded_memory = get_bits(word,2,2) == 1;
        self.executive_expanded_memory = get_bits(word,1,1) == 1;
        self.user_mode = get_bits(word,0,0) == 1;

        if self.user_mode == true && self.user < 2{
            eprintln!("Bug 1 on msr load ?");
        }

        //println!("Loading msr, user : {}",self.get_current_user_info_str());
    }

    pub(crate) fn get_msr_word(&self) -> u16{

        let mut msr_word:u16 = 0;

        set_bits(&mut msr_word, 13, 15, self.user as u16);
        set_bits(&mut msr_word, 9, 9, self.dma_protection as u16);
        set_bits(&mut msr_word, 8, 8, self.io_protection as u16);

        set_bits(&mut msr_word, 7, 7, self.defer_indirect_protection as u16);
        set_bits(&mut msr_word, 6, 6, self.user_page_protection as u16);
        set_bits(&mut msr_word, 5, 5, self.dma_map as u16);
        set_bits(&mut msr_word, 4, 4, self.user_data_map as u16);
        set_bits(&mut msr_word, 3, 3, self.executive_data_map as u16);
        set_bits(&mut msr_word, 2, 2, self.user_expanded_memory as u16);

        set_bits(&mut msr_word, 1, 1, self.executive_expanded_memory as u16);
        set_bits(&mut msr_word, 0, 0, self.user_mode as u16);

        msr_word

    }
}


#[derive(Clone,Copy)]
pub(super) struct MemFormatControlWord{
    pub(super) physical_page:u16,
    dirty_flag:bool,
    pub(super) execute_protected:bool,
    pub(super) read_protected:bool,
    pub(super) write_protected:bool
}

impl From<u16> for MemFormatControlWord {
    fn from(control_word: u16) -> Self {
        
        let physical_page = get_bits(control_word, 6, 15);
        let dirty_flag = get_bits(control_word,0,0) == 1;
        let execute_protected = get_bits(control_word,1,1) == 1;
        let read_protected = get_bits(control_word,2,2) == 1;
        let write_protected = get_bits(control_word,3,3) == 1;

        MemFormatControlWord{
            physical_page,
            dirty_flag,
            execute_protected,
            read_protected,
            write_protected,
        }
    }
}

impl From<&MemFormatControlWord> for u16 {
    fn from(mcw: &MemFormatControlWord) -> Self {
        let mut control_word = 0_u16;

        set_bits(&mut control_word,6,15,mcw.physical_page);

        set_bits(&mut control_word,0,0,mcw.dirty_flag as u16);
        set_bits(&mut control_word,1,1,mcw.execute_protected as u16);
        set_bits(&mut control_word,2,2,mcw.read_protected as u16);
        set_bits(&mut control_word,3,3,mcw.write_protected as u16);


        control_word
    }
}
impl MemFormatControlWord{
    
    pub(crate) fn get_shifted_pyhsical_page_adr(&self) -> u32{
        (self.physical_page as u32) << 10
    }
    
    pub(crate) fn is_invalid(&self) -> bool{
        self.execute_protected && self.read_protected && self.write_protected
    }

    pub(crate) fn is_valid(&self) -> bool{
        !self.is_invalid()
    }
}


#[cfg(test)]
mod tests {
    use std::io::stdout;
    use crate::assembler::Assembler;
    use crate::{instruction_decoder};
    use crate::instruction_identifier::{instruction_data_parser, InstructionIdentifier};
    use super::*;
    #[test]
    fn msr_encode_decode_test()  {
        let mut mpu = MemoryStatusRegister::new_with_logger((Rc::new(RefCell::new(Box::new(stdout())))));

        assert_eq!(mpu.get_msr_word(),1);

        mpu.user = 2;
        mpu.dma_protection = true;
        mpu.user_mode = true;
        mpu.executive_data_map = true;

        println!("{:#x}", mpu.get_msr_word());
        assert_eq!(mpu.get_msr_word(),0x9042);



    }


}