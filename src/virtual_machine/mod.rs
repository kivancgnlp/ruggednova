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

    pub(crate) tto_buffer : String,
    pub(crate) cpu_halted : bool,

    /// Power Fail flag of the CPU device (code 77). `SKPDN CPU` / `SKPDZ CPU` test it (3-110).
    /// Nothing sets it yet; a power-fail simulation would.
    pub(crate) power_fail : bool,

    /// Set by every stack push. Section 2.30 puts the overflow check "at the conclusion of all
    /// instructions and other operations which push elements onto the Stack" — once per
    /// instruction, not once per word — so SAVE, PST and a nested interrupt each get a single
    /// check after all their pushes are done.
    pub(crate) stack_push_occurred : bool,

    /// SAVE's roll-over exception (3-69): "If allocation of the block size n results in roll-over
    /// of the stack pointer, the resultant stack pointer is greater than the stack limit, but the
    /// overflow will still be detected." The unsigned SP < SL comparison cannot see that case, so
    /// the instruction reports it directly.
    pub(crate) force_stack_overflow : bool,

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


    /// Manual p. 3-69, "Stack Registers and Expanded Memory":
    ///
    /// > If a stack register has bit 0 set, pointing to an address in expanded memory, the actual
    /// > address will be in the lower 32k words of memory, i.e., address 107266 will cause the
    /// > stack operation to be performed at 007266.
    /// >
    /// > Microcode comparisons of stack pointer and stack limit compare all 16 bits.
    ///
    /// So bit 0 is stripped from the ADDRESS of a stack access when expanded memory is disabled,
    /// but the register itself keeps it, and the overflow comparison still sees all 16 bits.
    ///
    /// INS64 group H, H18 at 004245 tests exactly this, and the listing calls it out as a way to
    /// "cheat" the stack overflow check: it sets SP to T3 | 0x8000 with EM clear, pushes, and then
    /// reads the pushed word back from the LOW address T3-1. A 16-bit SP well above SL means no
    /// overflow is raised, while the data lands in page zero.
    fn stack_access_address(&self, pointer: u16) -> u16 {
        if self.is_expanded_memory_for_the_current_user() {
            pointer
        } else {
            pointer & 0x7fff
        }
    }

    pub(crate) fn push_a_single_word_to_the_stack(&mut self, data_word: u16) {
        // Roll-over is a documented case, not a bug (3-69), so this must wrap rather than panic.
        self.sp = self.sp.wrapping_sub(1);
        let adr = self.stack_access_address(self.sp);
        self.mapping_unit.write_word_to_memory(adr,data_word,true);

        // The fault is NOT raised here. Section 2.30 checks at the conclusion of the instruction,
        // which is what lets SAVE push its six words plus an n-word block and only then fault.
        self.stack_push_occurred = true;
    }

    /// Section 2.30 / 3-69. Call once, after an instruction has finished executing.
    ///
    /// "The Stack Pointer and the Stack Limit are treated as unsigned 16-bit integers and
    /// compared. If the Stack Pointer is less than the Stack Limit, a stack overflow condition
    /// exists and a trap is initiated."
    ///
    /// SP == SL is legal — it is the last usable word of the stack. INS64 group H, H16 at 004161
    /// relies on exactly that: it sets SP = 0o421 / SL = 0o420 and requires the first push (which
    /// leaves SP == SL) to succeed and the second to fault.
    ///
    /// Returns true when the trap was taken.
    pub(crate) fn check_for_stack_overflow_after_an_instruction(&mut self) -> bool {

        if !self.stack_push_occurred && !self.force_stack_overflow {
            return false;
        }

        let overflowed = self.force_stack_overflow || self.sp < self.sl;

        self.stack_push_occurred = false;
        self.force_stack_overflow = false;

        if overflowed {
            self.stack_overflow_trap();
        }

        overflowed
    }

    /// Section 2.30: "This trap sequence stores the address of the next instruction to be executed
    /// in memory location 44, and executes a JMP to the address stored in location 45."
    ///
    /// `ip` already points at the next instruction by the time this runs — each pushing
    /// instruction increments the PC before the check (see the PSH and PJS flows, 3-106).
    ///
    /// The trap does NOT switch to Executive Mode when the processor is in User Mode, and it does
    /// not touch Carry or Overflow.
    pub(crate) fn stack_overflow_trap(&mut self) {

        let return_address = self.ip;

        self.mapping_unit.set_stack_overflow_return_address(return_address);
        self.ip = self.mapping_unit.get_stack_overflow_handler_address();

        // INS64 H16 (listing page 0054) checks with SKPBZ CPU that ION was cleared by the trap,
        // and its companion test at H16E accepts ION still set on a MAPPED cpu. The 1666B is
        // mapped, and section 2.30 says nothing about ION, so ION is left alone here. Only the
        // Branch-and-Nest sequence clears it explicitly (section 2.27: "If the stack has
        // overflowed, ION is cleared and the Stack Overflow Trap Sequence is initiated").
    }

    /// Section 2.29, the Unimplemented Instruction Trap:
    ///
    /// > The Unimplemented Instruction Trap will occur upon execution of an instruction code that
    /// > is undefined or unimplemented (see Appendix D). Next, the address of the unimplemented
    /// > instruction will be stored in location 42, and JMP to the address stored in location 43
    /// > will occur. ... If the processor is in User Mode, the trap does not cause a switch to
    /// > Executive Mode.
    ///
    /// Note the contrast with the Stack Overflow Trap: that one stores the address of the NEXT
    /// instruction in location 44, this one stores the address of the offending instruction
    /// itself, so the handler can look at it. INS64 group Q relies on the difference — its handler
    /// does `ISZ 42` to step past the bad word before `JMP @42`.
    ///
    /// `instruction_address` is the address of the unimplemented instruction, not of whatever
    /// follows it.
    pub(crate) fn unimplemented_instruction_trap(&mut self, instruction_address: u16) {

        self.mapping_unit.set_unimplemented_instruction_address(instruction_address);

        let vector = self.mapping_unit.get_unimplemented_instruction_handler_address();
        self.ip = self.resolve_indirect_chain(vector);
    }

    /// Follow an indirect chain starting from an already-fetched word.
    ///
    /// Section 2.17: with expanded memory disabled, bit 0 of an address word is the indirect bit
    /// rather than part of the address, so a word with bit 0 set means "the address is in the word
    /// this points at". Section 1.6 faults a chain deeper than 16 levels.
    ///
    /// INS64 group Q needs this for the trap vector: at 006041 it builds `Q97 | 100000` and stores
    /// that in location 43, so reaching the handler requires resolving one level of indirection
    /// rather than jumping to 106053.
    fn resolve_indirect_chain(&mut self, first_word: u16) -> u16 {

        const MAX_INDIRECTION_LEVELS: usize = 16;   // section 1.6

        let mut word = first_word;

        if self.is_expanded_memory_for_the_current_user() {
            // With EM enabled the whole 16 bits are address; there is no indirect bit to follow.
            return word;
        }

        for _ in 0..MAX_INDIRECTION_LEVELS {
            if word & 0x8000 == 0 {
                return word;
            }
            word = self.mapping_unit.read_word_from_memory(word & 0x7fff, true);
        }

        eprintln!("Indirect chain deeper than {} levels while resolving a trap vector", MAX_INDIRECTION_LEVELS);
        word & 0x7fff
    }

    pub(crate) fn pop_a_single_word_from_the_stack(&mut self) -> u16 {

        let adr = self.stack_access_address(self.sp);
        let data_word = self.mapping_unit.read_word_from_memory(adr,true);
        self.sp = self.sp.wrapping_add(1);

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

    // The fifth word of the return block. Figures 3-9 and 3-10 label it "BIT 0 CARRY / BIT 1 OVFL",
    // and throughout this manual bit 0 is the MOST significant bit — so Carry is 0x8000 and Overflow
    // is 0x4000, not 0x0001 / 0x0002. Encode and decode are inverses either way, so SAVE/RTRN/POPB
    // round-trip regardless; the layout only becomes visible when guest code inspects the word
    // directly, or when a trace is compared against a real machine dump.
    const RETURN_BLOCK_CARRY_BIT: u16    = 0x8000; // manual bit 0
    const RETURN_BLOCK_OVERFLOW_BIT: u16 = 0x4000; // manual bit 1

    pub(crate) fn encode_carry_and_overflow(&self) -> u16  {
        let mut carry_and_overflow = 0_u16;
        if self.carry_flag {
            carry_and_overflow |= Self::RETURN_BLOCK_CARRY_BIT;
        }
        if self.overflow_flag {
            carry_and_overflow |= Self::RETURN_BLOCK_OVERFLOW_BIT;
        }

        carry_and_overflow

    }

    pub(crate) fn decode_carry_and_overflow(&mut self, compound : u16 )  {
        debug_assert!(compound & !(Self::RETURN_BLOCK_CARRY_BIT | Self::RETURN_BLOCK_OVERFLOW_BIT) == 0,
                      "unexpected bits set in the return-block carry/overflow word: {:#06x}", compound);

        self.carry_flag    = compound & Self::RETURN_BLOCK_CARRY_BIT    != 0;
        self.overflow_flag = compound & Self::RETURN_BLOCK_OVERFLOW_BIT != 0;


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
            tto_buffer: String::new(),
            cpu_halted: false,
            power_fail: false,
            stack_push_occurred: false,
            force_stack_overflow: false,
        }
    }
    
}
#[cfg(test)]
mod stack_overflow_trap {
    use super::*;

    /// Sets up the machine the way INS64 group H, H16 at 004161 does: SP just one word above SL,
    /// with a handler address installed in location 45.
    fn machine_with_a_nearly_full_stack() -> ExecutionContext {
        let mut ec = ExecutionContext::new();
        let mut mem = vec![0u16; 0x600];
        mem[0o45] = 0o4176;            // stack overflow trap handler, as H16 installs it
        ec.load_initial_memory(mem);
        ec.sp = 0o421;
        ec.sl = 0o420;
        ec.ip = 0o4173;
        ec
    }

    /// "If the Stack Pointer is less than the Stack Limit" — so SP == SL is the last legal word,
    /// not a fault. INS64 H16 pushes once expecting success ("SHOULD NOT GET STACK OVERFLOW")
    /// before pushing again expecting the trap.
    #[test]
    fn a_push_that_lands_on_the_stack_limit_is_legal() {
        let mut ec = machine_with_a_nearly_full_stack();

        ec.push_a_single_word_to_the_stack(0x1234);
        let trapped = ec.check_for_stack_overflow_after_an_instruction();

        assert_eq!(ec.sp, 0o420, "SP now equals SL");
        assert!(!trapped, "SP == SL must not fault");
        assert_eq!(ec.ip, 0o4173, "PC untouched");
    }

    /// The very next push takes SP below SL and must trap.
    #[test]
    fn a_push_below_the_stack_limit_traps() {
        let mut ec = machine_with_a_nearly_full_stack();

        ec.push_a_single_word_to_the_stack(0x1234);
        assert!(!ec.check_for_stack_overflow_after_an_instruction());

        ec.ip = 0o4174;                       // the PC is advanced before the check (3-106)
        ec.push_a_single_word_to_the_stack(0x5678);
        let trapped = ec.check_for_stack_overflow_after_an_instruction();

        assert!(trapped);
        assert_eq!(ec.ip, 0o4176, "JMP to the address in location 45");
        assert_eq!(ec.mapping_unit.read_word_from_memory(0o44, true), 0o4174,
                   "location 44 holds the address of the next instruction");
    }

    /// An instruction that pushes nothing is never checked, however sick the pointers look.
    #[test]
    fn no_push_means_no_check() {
        let mut ec = machine_with_a_nearly_full_stack();
        ec.sp = 0;                            // far below the limit
        assert!(!ec.check_for_stack_overflow_after_an_instruction());
        assert_eq!(ec.ip, 0o4173);
    }

    /// Section 2.30 checks "at the conclusion of" a pushing instruction, so one instruction that
    /// pushes six words gets one check, and a push that dips below SL and is undone within the
    /// same instruction never faults.
    #[test]
    fn the_check_is_per_instruction_not_per_word() {
        let mut ec = machine_with_a_nearly_full_stack();

        ec.push_a_single_word_to_the_stack(1);   // SP -> 420
        ec.push_a_single_word_to_the_stack(2);   // SP -> 417, below SL
        ec.pop_a_single_word_from_the_stack();   // SP -> 420 again
        ec.pop_a_single_word_from_the_stack();   // SP -> 421

        assert!(!ec.check_for_stack_overflow_after_an_instruction(),
                "only the pointer at the end of the instruction matters");
    }

    /// Manual p. 3-69: SAVE's roll-over "will still be detected" even though the resulting SP
    /// compares as greater than SL.
    #[test]
    fn save_roll_over_is_reported_even_though_sp_looks_healthy() {
        let mut ec = machine_with_a_nearly_full_stack();
        ec.sp = 0x8000;
        ec.sl = 0x0100;
        ec.force_stack_overflow = true;       // what SAVE sets when the allocation wraps
        ec.sp = 0xfff0;                       // wrapped: now far ABOVE the limit

        assert!(ec.check_for_stack_overflow_after_an_instruction(),
                "roll-over faults despite SP > SL");
        assert_eq!(ec.ip, 0o4176);
    }

    /// Manual p. 3-69: with expanded memory disabled, a stack register with bit 0 set addresses
    /// the low 32K — "address 107266 will cause the stack operation to be performed at 007266" —
    /// while the register keeps bit 0 and the SP/SL comparison still sees all 16 bits.
    ///
    /// INS64 group H, H18 at 004245 calls this "cheating" the stack overflow check.
    #[test]
    fn a_stack_pointer_with_bit_zero_set_addresses_the_low_32k() {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 0x600]);
        ec.sp = 0x8038;
        ec.sl = 0x0110;

        ec.push_a_single_word_to_the_stack(0x4321);

        assert_eq!(ec.sp, 0x8037, "the register keeps bit 0");
        assert_eq!(ec.mapping_unit.read_word_from_memory(0x0037, true), 0x4321,
                   "but the word lands in the low 32K");
        assert!(!ec.check_for_stack_overflow_after_an_instruction(),
                "0x8037 > 0x0110 as unsigned 16-bit values, so no overflow is raised");
    }
}
