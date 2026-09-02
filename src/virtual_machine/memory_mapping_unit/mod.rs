mod mmu_helper_types;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::io::{stdout, BufWriter, Error, Write};
use std::rc::Rc;
use crate::instruction_decoder::bit_utils::{get_bits, set_bits};
use crate::virtual_machine::memory_mapping_unit::mmu_helper_types::{MemFormatControlWord, MemoryStatusRegister, MemoryViolationsRegister};



//#[derive(Debug)]
pub(crate) struct MappingUnit {

    pub(crate) msr: MemoryStatusRegister,
    pub(crate) mvr: MemoryViolationsRegister,

    // The following fields are for emulating the execution model (not represent actual HW flags)
    mem : Vec<u16>,
    mem_maps : HashMap<(u8,u8), MemFormatControlWord>, // For tracking memory maps
    mem_maps_for_readmap_instruction : HashMap<u16, u16>, // these two can be merged in the future ( mem_maps and mem_maps_for_readmap_instruction)
    mem_adr_referenced_as_instruction : HashSet<u32>,
    mem_adr_referenced_as_data : HashSet<u32>,
    observed_mem_access_types : Vec<u16>,
    pub(crate) temporary_use_user_instruction_map_for_data_referencing: bool, // for MAPSI instructions
    pub(crate) temporary_use_user_data_map_for_data_referencing: bool, // for MAPSD instructions
    pub(crate) trace_phy_mem_accesses : bool,
    pub(crate) log_writer : Rc<RefCell<Box<dyn Write>>>,
    use_instruction_map : bool,

}

impl MappingUnit {



    fn check_mem_access_type(&mut self,physical_adr:u32, data:bool, write:bool) -> bool{

        let mut ok = true;

        /*
        if physical_adr == 0x1CC51{
            println!("Data access breakpoint");
        }

         */

        if data {
            if write && self.mem_adr_referenced_as_instruction.contains(&physical_adr){
                 eprintln!("Data write to a location that is previously accessed as instruction. Phy adr : {:#x}, user : {}", physical_adr, self.msr.get_current_user_info_str());
                //panic!("Data access to a location that is previously accessed as instruction. Phy adr : {:#x}, user : {}", physical_adr, self.get_current_user_info_str());
                //self.dump_observed_mem_access_stats();
                ok = false;
            }

            self.mem_adr_referenced_as_data.insert(physical_adr);
        }else {
            if self.mem_adr_referenced_as_data.contains(&physical_adr){
                eprintln!("Instruction access to a location that is previously accessed as data. Phy adr : {:#x}, user : {}", physical_adr, self.msr.get_current_user_info_str());
                //panic!("Instruction access to a location that is previously accessed as data. Phy adr : {:#x}, user : {}", physical_adr, self.get_current_user_info_str())
                //self.dump_observed_mem_access_stats();
                //ok = false;
            }
            self.mem_adr_referenced_as_instruction.insert(physical_adr);
        }

        if ok {
            let access_type = match (data, write) {
                (false, false) => 'I',
                (false, true) => 'i',
                (true, false) => 'R',
                (true, true) => 'W',
            };
            let user_info_char = self.msr.get_current_user_info_str();
            let ident_word = (access_type as u16) << 8 | user_info_char as u16;
            self.observed_mem_access_types[physical_adr as usize] = ident_word;
        }
        ok

    }

    pub(crate) fn use_instruction_map(&mut self, yes:bool){

        self.use_instruction_map = yes;
    }

    pub(crate) fn write_double_word_to_data_memory(&mut self, logical_address:u16, dw_data:u32){
        self.write_word_to_memory(logical_address,(dw_data >> 16) as u16,true); // First high word according to big endian
        self.write_word_to_memory(logical_address +1,(dw_data & 0xffff) as u16,true);

    }
    pub(crate) fn read_double_word_from_memory(&mut self, logical_address:u16) -> u32{
        let operand_h = self.read_word_from_memory(logical_address,true);
        let operand_l = self.read_word_from_memory(logical_address+1,true);

        let dw = ((operand_h as u32) << 16) + operand_l as u32;
        dw

    }

    pub(crate) fn read_word_from_memory(&mut self, logical_address:u16, mark_as_data : bool) -> u16{

        if self.temporary_use_user_instruction_map_for_data_referencing{
            return self.read_word_from_mem_using_instruction_map(logical_address,false,false);
            //return self.read_word_from_data_memory(logical_address,false);
        }

        if self.temporary_use_user_data_map_for_data_referencing{
            return self.read_word_from_data_memory(logical_address,false);
        }


        if self.use_instruction_map{
            self.read_word_from_mem_using_instruction_map(logical_address, mark_as_data, self.msr.is_executive())
        }else {
            self.read_word_from_data_memory(logical_address,self.msr.is_executive())
        }

    }

    pub(crate) fn write_word_to_memory(&mut self, logical_address:u16, data_word:u16, mark_as_data : bool){


        if self.temporary_use_user_instruction_map_for_data_referencing{
            self.write_word_to_mem_using_instruction_map(logical_address,data_word,false,false);
            //self.write_word_to_data_memory(logical_address,data_word,false);
            return;
        }

        if self.temporary_use_user_data_map_for_data_referencing{
            self.write_word_to_data_memory(logical_address,data_word,false);
            return;
        }


        if self.use_instruction_map{
            self.write_word_to_mem_using_instruction_map(logical_address, data_word,mark_as_data,self.msr.is_executive());
        }else {
            self.write_word_to_data_memory(logical_address, data_word,self.msr.is_executive());
        }

    }

    fn check_mem_access_violations(&mut self,logical_adr:u16, data:bool, write:bool, executive_mode:bool){
        let logical_id = get_bits(logical_adr,0,5) as u8;

        let (map,map_info) = self.determine_map_file_to_use(executive_mode,data);

        if let Some(map_i) = map {

            let cw = self.mem_maps.get(&(map_i,logical_id)).expect("Mem map coudn't be found during checking mem_access_violations");

            if data{
                if write && cw.write_protected{
                    eprintln!("Data write protection violated");
                    self.mvr.write_protection_error = true;
                }

                if !write && cw.read_protected{
                    eprintln!("Data read protection violated");
                    self.mvr.read_protection_error = true;
                }
            }

        }

    }
    fn read_word_from_data_memory(&mut self, logical_address:u16, executive_mode:bool) -> u16{

        let (physical_adr,map_info_str) = self.translate_logical_adr_to_physical_adr(logical_address,true,executive_mode);
        self.check_mem_access_violations(logical_address,true,false,executive_mode);
        let access_ok=self.check_mem_access_type(physical_adr, true, false);
        if !access_ok {
            eprintln!("Access type conflict for user {}, and logical adr : {:#x}",self.msr.user,logical_address);
            panic!("Access type conflict");
        }
        let word_read =self.mem[physical_adr as usize];

        if self.trace_phy_mem_accesses{
            writeln!(self.log_writer.borrow_mut(),"Loading data from physical adr : {:#x}, logical adr : {:#x}, read data : {:#x}, map info : {}", physical_adr, logical_address, word_read,map_info_str);
        }

        word_read

    }

    fn write_word_to_data_memory(&mut self, logical_address:u16, data_word:u16, executive_mode:bool){

        let (physical_adr,map_info_str) = self.translate_logical_adr_to_physical_adr(logical_address,true, executive_mode);
        self.check_mem_access_violations(logical_address,true,true,executive_mode);
        let access_ok = self.check_mem_access_type(physical_adr, true,true);

        if access_ok {

            if self.trace_phy_mem_accesses{
                writeln!(self.log_writer.borrow_mut(),"Storing data {:#x} to memory physical adr : {:#x}, logical adr : {:#x}, map info : {}", data_word, physical_adr, logical_address,map_info_str);
            }
            self.mem[physical_adr as usize] = data_word;
        }else {
            writeln!(self.log_writer.borrow_mut(),"Write data prevented {:#x} to memory physical adr : {:#x}, logical adr : {:#x}, map info : {}", data_word, physical_adr, logical_address,map_info_str);

        }





    }

    pub(crate) fn read_word_from_mem_using_instruction_map(&mut self, logical_adr:u16, mark_as_data:bool, executive:bool) ->u16{

        let (physical_adr,map_info_str) = self.translate_logical_adr_to_physical_adr(logical_adr,false,executive);
        let access_ok  =self.check_mem_access_type(physical_adr, mark_as_data,false);

        if !access_ok {
            writeln!(self.log_writer.borrow_mut(),"Access type conflict for user {}, and logical adr : {:#x}",self.msr.user,logical_adr);
            panic!("Access type conflict");
        }
        let read_word = self.mem[physical_adr as usize];

        if self.trace_phy_mem_accesses{
            writeln!(self.log_writer.borrow_mut(),"Loading data from instruction memory, from physical adr : {:#x}, logical adr : {:#x}, read data : {:#x}, map info : {}", physical_adr, logical_adr, read_word,map_info_str);
        }

        read_word

    }

    pub(crate) fn write_word_to_mem_using_instruction_map(&mut self, logical_adr: u16, data_word: u16, mark_as_data:bool, executive:bool) {
        let (physical_adr,map_info_str) = self.translate_logical_adr_to_physical_adr(logical_adr,false, executive);
        let access_ok = self.check_mem_access_type(physical_adr, mark_as_data,true);

        if access_ok {
            self.mem[physical_adr as usize] = data_word;
        }else {
            writeln!(self.log_writer.borrow_mut(),"Access type conflict for user {}, and logical adr : {:#x}",self.msr.user,logical_adr);
            //self.trace_phy_mem_accesses = true;
            //panic!("Access type conflict");
        }


        if self.trace_phy_mem_accesses{
            writeln!(self.log_writer.borrow_mut(),"Storing data {:#x} to instrucion memory physical adr : {:#x}, logical adr : {:#x}, map info : {}", data_word, physical_adr, logical_adr,map_info_str);
        }

    }

    pub(crate) fn load_initial_memory(&mut self, input_word_mem: Vec<u16>) {

        for (index,word) in input_word_mem.iter().enumerate(){
            self.mem[index] = *word;
        }

    }

    fn prepare_protected_io_device_list_string(group_base:u8, mut device_bits: u16) -> String {
        let mut devices_string = String::new();
        for i in 0..16{
            if device_bits & 1 == 1{
                devices_string.push_str(format!("{:#o},", group_base + i).as_str());
            }

            device_bits >>= 1;
        }

        devices_string
    }

    pub(crate) fn add_single_mapping_for_test_purpose(&mut self, user:u8, logical:u8, phy_mapping:u16){
        let mem_cw = MemFormatControlWord::from(phy_mapping);

        self.mem_maps.insert((user, logical), mem_cw);
        
    }

    pub(crate) fn get_single_mapping_naive(&self, access_word:u16) -> u16{

        self.mem_maps_for_readmap_instruction[&access_word]

    }

    pub(crate) fn add_single_mapping(&mut self, access_word:u16, control_word:u16){

        //println!("access word : {:#x}, Control word : {:#x}", access_word,control_word);
        self.mem_maps_for_readmap_instruction.insert(access_word, control_word);

        let io_format = get_bits(access_word,6,6) == 1;

        if !io_format {
            let logical_page = get_bits(access_word,0,5);
            let map_file = get_bits(access_word,13,15);

            let mem_control_word = MemFormatControlWord::from(control_word);

            let map_file_info = match map_file {
                0 => "DMA",
                1 => "Executive",
                _ => "User"
            };

            let map_file_info_str = format!("{:#x} ({})", map_file, map_file_info);

            let physical_info_str;
            if logical_page == mem_control_word.physical_page {
                physical_info_str = format!("1:1 mapping (physical offset : {:#08x})", mem_control_word.get_shifted_pyhsical_page_adr());
            }else {
                physical_info_str = format!("physical page : {:#04x} (physical offset : {:#08x})", mem_control_word.physical_page, mem_control_word.get_shifted_pyhsical_page_adr());
            }


            self.mem_maps.insert((map_file as u8, logical_page as u8), mem_control_word);

            let mut protection_summary_str = String::new();

            if mem_control_word.is_invalid() {
                protection_summary_str.push_str("mapping intentionally invalidated ")
            } else {
                if mem_control_word.execute_protected {
                    protection_summary_str.push_str("execute_protected, ")
                }
                if mem_control_word.read_protected {
                    protection_summary_str.push_str("read_protected, ")
                }
                if mem_control_word.write_protected {
                    protection_summary_str.push_str("write_protected, ")
                }
            }


            writeln!(self.log_writer.borrow_mut(), "Adding mapping : , map file : {map_file_info_str}, Logical page : {logical_page:#04x}, {physical_info_str}, {protection_summary_str}");





        }else {
            let group = get_bits(access_word,4,5) as u8;
            let io_file = get_bits(access_word,13,15) as u8;

            let io_group_base = match group {
                0 => 0,
                1 => 0o20,
                2 => 0o40,
                3 => 0o60,
                _ => unreachable!()
            };

            let protected_devices_string = Self::prepare_protected_io_device_list_string(io_group_base,control_word);
            writeln!(self.log_writer.borrow_mut(),"Folowing IO devices are protected from user {} : {}",io_file,protected_devices_string);

             // TODO : Bunu kimse kullanmiyor, test ve kontrol edilmeli
        }


    }

    fn determine_map_file_to_use(&self, executive_mode : bool, data_access : bool) -> (Option<u8>, String){

        if executive_mode && !data_access {
            return (None, "Using 1:1 mapping for address translation (executive instruction access)".to_string()); // Executive instruction access always 1:1
        };

        if executive_mode && data_access {
            if self.msr.executive_data_map{
                return (Some(1),"Using executive data map (map file 1) for address translation (executive data access via data map)".to_string());
            }else{
                // executive data map not enabled
                return (None,"Using 1:1 mapping for address translation (executive direct data access)".to_string());
            }
        }

        //Here we are in user mode
        let mut file = self.msr.user;

        if file % 2 == 0 && data_access && self.msr.user_data_map{ // increment file if user data map enabled for user 2,4,6
            if self.temporary_use_user_instruction_map_for_data_referencing == false{ // if MAPSI in effect it has higher priority
                eprintln!("User data map usage is not tested");
                file +=1;
            }

        }

        let access_info = if data_access{"User data access"} else {"User instruction access"};
        (Some(file), format!("Using file {} for {}",file,access_info))

    }

    fn translate_logical_adr_to_physical_adr(&self, logical_adr:u16, data_access : bool, executive_mode : bool) -> (u32, String) {

        let logical_id = get_bits(logical_adr,0,5) as u8;
        let logical_adr_offset_part = get_bits(logical_adr,6,15) ;

        let (map_to_use, map_info_string) = self.determine_map_file_to_use(executive_mode, data_access);

        let physical_adr_base:u32;

        if let Some(map_file) = map_to_use {
            let cw = self.mem_maps.get(&(map_file,logical_id)).expect("Mapping not found");
            physical_adr_base = cw.get_shifted_pyhsical_page_adr();
        }else {
            physical_adr_base = (logical_id as u32) << 10; // this case means a direct (1:1) map, so adr upper part is same os logical adr upper part
        }

        let physical_adr = physical_adr_base + logical_adr_offset_part as u32;


        (physical_adr,map_info_string)

    }
    pub(crate) fn get_interrupt_stack(&self) -> (u16, u16) {

        let system_data_table_adr = self.mem[3];

        let irq_stack_pointer = self.mem[(system_data_table_adr + 2) as usize];
        let irq_stack_limit = self.mem[(system_data_table_adr + 3) as usize];
        (irq_stack_pointer, irq_stack_limit)

    }

    pub(crate) fn get_icnt(&self) -> u16 {

        let system_data_table_adr = self.mem[3];

        let icnt = self.mem[(system_data_table_adr + 1) as usize];
        
        icnt
    }

    pub(crate) fn set_icnt(&mut self, icnt: u16) {

        let system_data_table_adr = self.mem[3];

        self.mem[(system_data_table_adr + 1) as usize] = icnt;

        
    }

    pub(crate) fn get_executive_stack(&self) -> (u16, u16) {

        let system_data_table_adr = self.mem[3];

        let ex_stack_pointer = self.mem[(system_data_table_adr + 4) as usize];
        let ex_stack_limit = self.mem[(system_data_table_adr + 5) as usize];
        (ex_stack_pointer, ex_stack_limit)

    }

    pub(crate) fn get_trap_vector(&self, index:u8) -> u16 {

        let trap_service_table_adr = self.mem[2];

        self.mem[trap_service_table_adr as usize + index as usize]

    }

    fn dump_valid_maps_for_the_current_user(&self){

        let map_id = if !self.msr.user_mode {1} else {self.msr.user};

        for i in 0..64{
            let res = self.mem_maps.get(&(map_id, i));

            if let Some(p) = res {
                if p.is_valid(){
                    println!("logical id {} corresponds : {:#x}", i, p.get_shifted_pyhsical_page_adr());
                }

            }else {
                break;
            }
        }


    }
    pub(crate) fn dump_mem(&self) ->Result<(), Error>{

        println!("Dumping memory to mem.bin file");
        let mut file_writer = BufWriter::new(File::create("mem.bin")?);
        
        for word in self.mem.iter() {
            file_writer.write_all(word.to_be_bytes().as_slice())?;
        }
       
        
        Ok(())
    }

    pub(crate) fn dump_observed_mem_access_stats(&self) ->Result<(), Error>{

        println!("Dumping observed_mem_access_stats to mem_acc_type.bin file");
        let mut file_writer = BufWriter::new(File::create("mem_acc_type.bin")?);

        for word in self.observed_mem_access_types.iter() {
            file_writer.write_all(word.to_be_bytes().as_slice())?;
        }


        Ok(())
    }

    pub(crate) fn dump_mem_maps(&self) ->Result<(), Error>{

        println!("Dumping memory to maps.txt file");
        let mut file_writer = BufWriter::new(File::create("maps.txt")?);

        for ((user,logical),control_word) in self.mem_maps.iter() {
            let cw:u16 = u16::from(control_word);
            writeln!(&mut file_writer, "{:x}\t{:x}\t{:?}",user,logical, cw)?;
        }


        Ok(())
    }

    pub(crate) fn get_interrupt_vector(&self, device_id:u16) -> (u16, bool) {
        let interrupt_service_table_adr = self.mem[1];
        let target_adr = self.mem[(interrupt_service_table_adr+device_id)as usize];

        let branch_and_nest_isr = target_adr & 0x8000 == 0x8000;
        let target_adr = target_adr & 0x7fff;
        (target_adr,branch_and_nest_isr)
        
    }

    pub(crate) fn get_cmask(&self) -> u16 {
        
        let system_data_table_adr = self.mem[3];
        let cmask = self.mem[(system_data_table_adr+5)as usize];

        cmask

    }

    pub(crate) fn read_from_physical_memory_for_debug_or_test(&self, adr:u32) -> u16 {
        self.mem[adr as usize]
    }

    pub(crate) fn write_to_physical_memory_for_debug_or_test(&mut self, adr:u32,data_word:u16)  {
        self.mem[adr as usize] = data_word;
    }
    
    pub(crate) fn display_special_locations(&self) ->Result<(), Error>{

        let interrupt_service_table_adr = self.mem[1];
        let trap_service_table_adr = self.mem[2];
        let system_data_table_adr = self.mem[3];

        let mut out_stream = BufWriter::new(fs::File::create("../../../special_locations_info.txt")?);


        writeln!(out_stream,"Interrupt vector table ( address : {:#x}) :", interrupt_service_table_adr)?;
        writeln!(out_stream,"")?;

        for i in 0..64{
            let target_adr = self.mem[(interrupt_service_table_adr+i)as usize];
            let branch_and_nest_isr = target_adr & 0x8000 == 0x8000;
            let target_adr = target_adr & 0x7fff;
            writeln!(out_stream,"ISR adr for device {:#4o} is {:#x}. Branch and nest flag : {} ",i, target_adr,branch_and_nest_isr)?;
        }

        writeln!(out_stream,"")?;
        writeln!(out_stream,"Trap address table ( address : {:#x}) :", trap_service_table_adr)?;
        writeln!(out_stream,"")?;

        for i in 0..16{
            writeln!(out_stream,"TRAP adr for {:#4o} is {:#x} ",i,self.mem[(trap_service_table_adr+i) as usize])?;
        }

        writeln!(out_stream,"")?;
        writeln!(out_stream,"System data table ( address : {:#x}) :", system_data_table_adr)?;
        writeln!(out_stream,"")?;

        const ST_LOC_NAMES: [&str; 6] = ["Map Status Register (MSR)","Interrupt Nest Count (ICNT)","Interrupt Stack Pointer (ISP)","Interrupt Stack Limit (ISL)","Executive Stack Pointer (XSP)","Executive Stack Limit (XSL)"];

        for i in 0..6{
            writeln!(&mut out_stream, "{} : {:#x}", ST_LOC_NAMES[i], self.mem[(system_data_table_adr as u16 + i as u16) as usize])?;
        }

        Ok(())
    }

    pub(crate) fn display_covered_instructions(&self){

        let mut ins_covered: Vec<u32> = self.mem_adr_referenced_as_instruction.iter().copied().collect();
        ins_covered.sort();

        println!("Covered instruction ranges :");
        let mut prev:u32 = 0;

        for i in ins_covered{
            if prev + 1 != i {
                println!("{:#x}",i);
            }

            prev = i;
        }

        let mut data_covered: Vec<u32> = self.mem_adr_referenced_as_data.iter().copied().collect();
        data_covered.sort();

        println!("Covered data ranges :");
        prev = 0;
        for i in data_covered{
            if prev + 1 != i {
                println!("{:#x}",i);
            }

            prev = i;
        }



    }

    pub(crate) fn set_log_writer(&mut self, log_writer: Rc<RefCell<Box<dyn Write>>>) {
        self.log_writer = log_writer.clone();
        self.msr.log_writer = log_writer.clone();
    }

    pub fn new()->Self{

        let mut memory = Vec::<u16>::with_capacity(1024*1024);
        memory.resize(1024*1024, 0);

        let mut memory_stats = Vec::<u16>::with_capacity(1024*1024);
        memory_stats.resize(1024*1024, 0);

        MappingUnit{
            mem: memory,
            mem_maps: HashMap::new(),
            mem_maps_for_readmap_instruction: Default::default(),
            mem_adr_referenced_as_instruction: Default::default(),
            mem_adr_referenced_as_data: Default::default(),
            observed_mem_access_types: memory_stats,
            temporary_use_user_instruction_map_for_data_referencing: false,
            temporary_use_user_data_map_for_data_referencing: false,
            trace_phy_mem_accesses: false,
            log_writer: Rc::new(RefCell::new(Box::new(stdout()))), // default log writer is console for mem access logs
            use_instruction_map: false,
            msr: MemoryStatusRegister::new_with_logger(Rc::new(RefCell::new(Box::new(stdout())))),
            mvr: MemoryViolationsRegister::new(),
        }
    }




}
