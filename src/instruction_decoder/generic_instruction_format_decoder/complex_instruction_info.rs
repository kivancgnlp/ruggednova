// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::bit_utils::{get_bits, set_bits};
use crate::virtual_machine::{complex_instruction_executer, ExecutionContext};

pub(crate) fn explain(mnemonic : &str, execution_context: Option<&mut ExecutionContext>) -> Option<String> {

    let mut unable = false;
    let info_str = match mnemonic {
        "BAM" => "AC0 : Constant to add, AC1 : Number of words, AC2 : Src address, AC3 : Dst address",
        "ZAP" => "AC0 : Constant to set, AC1 : Number of words, AC2 : Dst address,",
        "WRMAP" | "RDMAP" => "AC0 : RMU access word format, AC1 : Number of words to transfer, AC2 : Dst address",
        "WRWRD" |  "RDWRD" =>  "AC0 : RMU access word format, AC1 : Control word",
        "DSPD" => "AC1 --> Control panel data lights",
        "EXMAP" => "Enables executive data map",
        "DXMAP" => "Disables executive data map",
        "STEM" => "Enables 64K address space and disables multi level indirection",
        "CLEM" => "Disables 64K address space and enables multi level indirection",
        "TCO" => "Skip if no overflow ( clear overflow after )",
        "INTEN" => "Interrupts enable",
        "INTDS"=> "Interrupts disable",
        "IORST" => "All I/O devices are set idle by clearing all Busy and Done flags. The 16-bit priority mask is set to zero, and the specified control function is performed.",
        "STIBN" => "Set the interrupt Branch and Nest flag, enabling branching interrupt sequences.",
        "CLIBN" => "Clear the interrupt Branch and Nest flag, disabling branching interrupt sequences.",
        "CLRD" => "Blanks Data Display",
        "PRT" => "Return by popping PC from stack",
        "RTRN" => "Pop block and return",
        "MAPSI" => "Temporarily use user instruction map for accessing data (for next instruction)",
        "MAPSD" => "Temporarily use user data map for accessing data (for next instruction)",
        "POPB" => "Pop block",
        "RTFNI" => "RETURN FROM NESTED INTERRUPT",
        "HALT"  => "Stop execution",
        "ECALL" => "Executive service call",
        "Skip if ION clear" => "Skips if interrupts disabled",
        "PST" => "Push status",
        "CMVR" => "CLEAR MAP VIOLATION REGISTER",
        "CDMA" => "CLEAR DMA VIOLATION",
        "STISZ" => "INCREMENT TOP ELEMENT OF STACK, SKIP IF ZERO",
        "EUB" => "EXECUTIVE TO USER BRANCH",
        "ABD" => "ADD TO BOTTOM OF DEQUE",
        "ATD" => "ADD TO TOP OF DEQUE",
        "RBD" => "REMOVE FROM BOTTOM OF DEQUE",
        "RTD" => "REMOVE FROM TOP OF DEQUE",
        "MOVBT" => "MOVE BYTE STRING WITH TERMINATOR",
        "MOVB" => "MOVE BYTE STRING",
            _ => {
            unable = true;
            "?"}
    };

    if let Some(ec) = execution_context {

        let mut auto_increment_ip = true;

        match mnemonic {

            "EXMAP" => {
                println!("Enabling executive data map (EXMAP)");
                ec.mapping_unit.msr.executive_data_map = true;
                //ec.mapping_unit.trace_phy_mem_accesses = true;
            }

            "DXMAP" => {
                ec.mapping_unit.msr.executive_data_map = false;
                println!("Disabling executive data map (DXMAP)");
                //ec.mapping_unit.trace_phy_mem_accesses = false;
            }

            "WRWRD" => {
                let access_word = ec.ac[0];
                let control_word = ec.ac[1];
                ec.mapping_unit.add_single_mapping(access_word,control_word);
            }

            "RDWRD" => {
                let access_word = ec.ac[0];
                let control_word = ec.mapping_unit.get_single_mapping_naive(access_word);

                ec.ac[1] = control_word;
            }

            "WRMAP" => {
                //AC0 : RMU access word format, AC1 : Number of words to transfer, AC2 : src address"
                let mut access_word = ec.ac[0];
                let number_of_words = ec.ac[1];
                let src_adr = ec.ac[2];

                let mut logical_page_id = get_bits(access_word,0,5); // By design WRMAP increments the page id in the access word in each iteration ( also applicable to IO format)

                for i in 0..number_of_words {
                    let control_word = ec.mapping_unit.read_word_from_memory(src_adr + i,true);
                    //println!("Reading control word from adr {:#x} : {:#x}",ec.mapping_unit.translate_logical_adr_to_physical_adr(src_adr + i),control_word);
                    ec.mapping_unit.add_single_mapping(access_word,control_word);

                    logical_page_id += 1;
                    set_bits(&mut access_word, 0, 5, logical_page_id);
                    ec.ac[0] = access_word;
                }

                ec.ac[1] = 0; // The number of words transferred should decrement at the end
                ec.ac[2] += number_of_words;

            }

            "RDMAP" => {
                //AC0 : RMU access word format, AC1 : Number of words to transfer, AC2 : src address"
                let mut access_word = ec.ac[0];
                let number_of_words = ec.ac[1];
                let src_adr = ec.ac[2];

                let mut logical_page_id = get_bits(access_word,0,5); // By design WRMAP increments the page id in the access word in each iteration ( also applicable to IO format)

                for i in 0..number_of_words {
                    let control_word = ec.mapping_unit.get_single_mapping_naive(access_word);
                    //println!("Reading control word from adr {:#x} : {:#x}",ec.mapping_unit.translate_logical_adr_to_physical_adr(src_adr + i),control_word);
                     ec.mapping_unit.write_word_to_memory(src_adr + i,control_word,true);

                    logical_page_id += 1;
                    set_bits(&mut access_word, 0, 5, logical_page_id);
                    ec.ac[0] = access_word;
                }

                ec.ac[1] = 0; // The number of words transferred should decrement at the end
                ec.ac[2] += number_of_words;

            }

            "STEM" => {
                if ec.mapping_unit.msr.user_mode {
                    ec.mapping_unit.msr.user_expanded_memory = true;
                }else {
                    ec.mapping_unit.msr.executive_expanded_memory = true;
                }
            }

            "CLEM" => {
                if ec.mapping_unit.msr.user_mode {
                    ec.mapping_unit.msr.user_expanded_memory = false;
                }else {
                    ec.mapping_unit.msr.executive_expanded_memory = false;
                }
            }

            "BAM" => {
                //"AC0 : Constant to add, AC1 : Number of words, AC2 : Src address, AC3 : Dst address"
                let constant_to_add = ec.ac[0];
                let number_of_word = ec.ac[1];
                let src_adr =  ec.ac[2];
                let dst_adr =  ec.ac[3];

                for i in 0..number_of_word {
                    let mut word = ec.mapping_unit.read_word_from_memory(src_adr + i,true);
                    word = word.wrapping_add(constant_to_add);
                    ec.mapping_unit.write_word_to_memory(dst_adr + i, word,true);
                }

                //ec.mapping_unit.dump_mem();
            }

            "TCO" =>{
                if ec.overflow_flag == false {
                    ec.ip += 1;
                }
                ec.overflow_flag = false;
            }

            "INTDS" => {
                ec.ion = false;
                println!("Interrupts disabled (INTDS)");
            }

            "INTEN" => {
                ec.ion = true;
                println!("Interrupts enabled (INTEN)");
            }

            "IORST" => {
                // Nothing here (yet)
            }

            "STIBN" => {
                println!("interrupt Branch and Nest (IBN) enabled");
                ec.ibn = true;
            }

            "CLIBN" => {
                println!("interrupt Branch and Nest (IBN) disabled");
                ec.ibn = false;
            }

            "CLRD" => {
                // Blanks Data Display ( not functional yet maybe implemented with DSPD )
            }

            "PRT" => {

                let return_adr = ec.pop_a_single_word_from_the_stack();
                println!("PRT return to {:#x }, call history : {:?},",return_adr,ec.call_stack_debug_information.pop_front());


                ec.ip = return_adr;
                auto_increment_ip = false;
            }

            "RTRN" => {

                println!("RTRN return to {:#x}, call depth : {}, history : ({:?})",
                         ec.mapping_unit.read_word_from_memory(ec.fp + 6,true),
                         ec.call_stack_debug_information.len(),
                         ec.call_stack_debug_information.pop_front());

                ec.sp = ec.fp;

                ec.fp = ec.pop_a_single_word_from_the_stack();
                let carry_and_overflow = ec.pop_a_single_word_from_the_stack();ec.decode_carry_and_overflow(carry_and_overflow);
                ec.ac[0] = ec.pop_a_single_word_from_the_stack();
                ec.ac[1] = ec.pop_a_single_word_from_the_stack();
                ec.ac[2] = ec.pop_a_single_word_from_the_stack();
                ec.ac[3] = ec.pop_a_single_word_from_the_stack();
                ec.ip = ec.pop_a_single_word_from_the_stack();

                auto_increment_ip = false;
            }

            "POPB" => {

                // Bunu yukardaki ile tek farkı, bunda IP pop etmiyor

                //Alternate implementation 1

                let carry_and_overflow = ec.mapping_unit.read_word_from_memory(ec.fp + 1,true);ec.decode_carry_and_overflow(carry_and_overflow);
                ec.ac[0] = ec.mapping_unit.read_word_from_memory(ec.fp + 2,true);
                ec.ac[1] = ec.mapping_unit.read_word_from_memory(ec.fp + 3,true);
                ec.ac[2] = ec.mapping_unit.read_word_from_memory(ec.fp + 4,true);
                ec.ac[3] = ec.mapping_unit.read_word_from_memory(ec.fp + 5,true);
                ec.sp = ec.fp + 6;
                ec.fp = ec.mapping_unit.read_word_from_memory(ec.fp,true);
                //Alternate implementation 2
                /*
                ec.sp = ec.fp;
                ec.fp = ec.pop_a_single_word_from_the_stack();
                let carry_and_overflow = ec.pop_a_single_word_from_the_stack();ec.decode_carry_and_overflow(carry_and_overflow);
                ec.ac[0] = ec.pop_a_single_word_from_the_stack();
                ec.ac[1] = ec.pop_a_single_word_from_the_stack();
                ec.ac[2] = ec.pop_a_single_word_from_the_stack();
                ec.ac[3] = ec.pop_a_single_word_from_the_stack();

                 */
            }

            "RTFNI" => {
                //ec.dump_stack();
                //ec.mapping_unit.dump_mem();
                //ec.mapping_unit.display_special_locations();
                //ec.mapping_unit.display_covered_instructions();

                const STACK_OVERFLOW_TEST:bool = false;

                if STACK_OVERFLOW_TEST {

                    println!("Simulation stack overflow");
                    //ec.mapping_unit.dump_mem();
                    ec.ip = ec.mapping_unit.read_word_from_memory(0o45,true);
                    auto_increment_ip = false;

                }else {
                    println!("executing RTFNI instruction");

                    let cmask = ec.pop_a_single_word_from_the_stack();
                    ec.mapping_unit.write_word_to_memory(5,cmask,true); // this updates location 5 as described in document

                    ec.interrupt_priority_mask = cmask;
                    ec.ip = ec.pop_a_single_word_from_the_stack();

                    let msr = ec.pop_a_single_word_from_the_stack();

                    let sp= ec.pop_a_single_word_from_the_stack(); // SP will be changed after these intructions so keep copy of them and update later
                    let sl= ec.pop_a_single_word_from_the_stack();

                    ec.sp = sp;
                    ec.sl = sl;

                    ec.load_msr_word(msr);

                    auto_increment_ip = false;
                    ec.call_stack_debug_information.pop_front();
                }

            }



            "MAPSI" => {
                ec.temporarily_activate_user_instruction_map_for_data_referencing(ec.ip + 1);
            }

            "MAPSD" => {

                ec.temporarily_activate_user_data_map_for_data_referencing(ec.ip + 1);

            }

            "ZAP" => {
                //AC0 : Constant to set, AC1 : Number of words, AC2 : Dst address,
                let constant_to_set = ec.ac[0];
                let number_of_words = ec.ac[1];
                let dst_adr = ec.ac[2];

                for i in 0..number_of_words {
                    ec.mapping_unit.write_word_to_memory(dst_adr + i , constant_to_set,true);
                }
            }

            "ECALL" => {

                ec.call_stack_debug_information.push_back(format!("ECALL from {:#x}",ec.ip));

                let vector = ec.mapping_unit.get_trap_vector(1);

                let temp1_msr = ec.mapping_unit.msr.get_msr_word();

                ec.mapping_unit.msr.executive_data_map = false;
                // Burada şöyle bir madde var (set expanded memory flag to XEM) ama anlamlı değil şu anki durumda çünkü bizim mapping_unit executive için ayrı EM flag tutuyor (XEM)
                let temp2_sp = ec.sp;
                let temp3_sl = ec.sl;

                if ec.mapping_unit.msr.user_mode { // switching from user mode to executive mode here
                    println!("Switching to executive stack by ECALL instruction. Calling user : {}, user SP : {:#x}",ec.mapping_unit.msr.user,ec.sp);
                    ec.mapping_unit.msr.user_mode = false; // switch to executive mode
                    let (x_sp,x_sl) = ec.mapping_unit.get_executive_stack();
                    ec.sp = x_sp;
                    ec.sl = x_sl;
                }

                ec.push_a_single_word_to_the_stack(temp3_sl);
                ec.push_a_single_word_to_the_stack(temp2_sp);
                ec.push_a_single_word_to_the_stack(temp1_msr);
                ec.push_a_single_word_to_the_stack(ec.ip + 1);
                let cmask = ec.mapping_unit.read_word_from_memory(5,true);
                ec.push_a_single_word_to_the_stack(cmask);

                ec.ip = vector;
                auto_increment_ip = false;



            },

            "Skip if ION clear" => {
                if ec.ion == false {
                    ec.ip += 1; // Alttaki artırma ile beraber 2 artacak ( sonraki inst gecilecek eğer ion değilse )
                }
            },

            "DSPD" => {
                println!("Panel lights : {:#x}",ec.ac[1]);
            }

            "PST" => {
                ec.push_status();
            }

            "CMVR" => {
                println!("Clearing map violation register");
                ec.mapping_unit.mvr.clear();
            }

            "CDMA" => {
                println!("Nothing to do for CDMA implemented in simulation right now");
            }

            "ABD" => {
                //ADD TO BOTTOM OF DEQUE
                let result = complex_instruction_executer::deque_add_to_bottom(ec);
                
                if result {
                    ec.ip += 2; // Normal exit : increase by 2
                    auto_increment_ip = false;
                }
            }

            "ATD" => {
                //ADD TO TOP OF DEQUE
                let result = complex_instruction_executer::deque_add_to_top(ec);

                if result {
                    ec.ip += 2; // Normal exit : increase by 2
                    auto_increment_ip = false;
                }
            }
            
            "RBD" => {
                // REMOVE FROM BOTTOM OF DEQUE
                let result =complex_instruction_executer::deque_remove_from_bottom(ec);

                if let Some(element) = result{
                    ec.ac[0] = element;
                    ec.ip += 2; // if not empty increase by 2
                    auto_increment_ip = false;
                }

            }

            "RTD" => {
                // REMOVE FROM TOP OF DEQUE
                let result = complex_instruction_executer::deque_remove_from_top(ec);

                if let Some(element) = result{
                    ec.ac[0] = element;
                    ec.ip += 2; // if not empty increase by 2
                    auto_increment_ip = false;
                }

            }

            "STISZ" => {
                let popped_word = ec.pop_a_single_word_from_the_stack(); // just checking the top element without sp change
                let incremented_word = popped_word.wrapping_add(1);
                ec.push_a_single_word_to_the_stack(incremented_word);

                println!("incremented_word: {:#x}",incremented_word);

                if incremented_word == 0{
                    ec.ip += 2; // if zero skip
                    auto_increment_ip = false;
                }// else will auto increment by 1
            }

            "EUB" => {
                let adr = ec.ac[0];

                let uadr = ec.mapping_unit.read_word_from_memory(adr,true);
                let usp = ec.mapping_unit.read_word_from_memory(adr + 1,true);
                let umsr = ec.mapping_unit.read_word_from_memory(adr + 2,true);
                let usl = ec.mapping_unit.read_word_from_memory(adr + 3,true);

                ec.mapping_unit.msr.set_msr_word(umsr);
                ec.sp = usp;
                ec.sl = usl;

                debug_assert!(ec.mapping_unit.msr.user_mode == true,"Some sanity check");
                ec.mapping_unit.msr.user_mode = true;
                //TODO : Not know how to handle now but clear map violation register
                ec.ip = uadr;
                auto_increment_ip = false;

            }

            "MOVBT" => {
                
                complex_instruction_executer::move_byte_string_with_terminator(ec);
            }

            "MOVB" => {

                complex_instruction_executer::move_byte_string(ec);
            }


            "HALT" => {
                ec.cpu_halted = true;
                println!("Halt executed. TTO buffer : {}",ec.tto_buffer);
            }


            _ => {
                //ec.mapping_unit.dump_mem();
                todo!("Unimplemented mnemonic {mnemonic} (complex instructions info module)");
            }
        }

        if auto_increment_ip {
            ec.ip += 1;
        }

    }

    if !unable {
        Some(format!("{} ({})",mnemonic, info_str))
    }else {
        None
    }

}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use super::*;

    #[test]
    fn rtfni_after_ecall_test()  {
        let mut ec = ExecutionContext::new();

        let mut mem_vec = vec![0; 0x200];

        mem_vec[2] = 0x50; // trap_service_table_adr
        mem_vec[3] = 0x60; // system_data_table_adr

        mem_vec[0x51] = 0x1234; //ECALL vector

        mem_vec[0x64] = 0x200; //executive SP
        mem_vec[0x65] = 0x190; //executive SL


        ec.load_initial_memory(mem_vec);

        ec.mapping_unit.msr.user_mode = true;
        ec.mapping_unit.msr.user = 2;
        ec.mapping_unit.msr.user_expanded_memory = true;
        ec.mapping_unit.add_single_mapping_for_test_purpose(2,0,1);

        ec.fp = 0x100;
        ec.sp = 0x100;
        ec.sl = 0x90;
        ec.ip = 0;
        
        let ins_word = ec.read_word_from_mem_using_instruction_map_pc_relative(0,false);
        

        explain("ECALL",Some(&mut ec));
        explain("RTFNI",Some(&mut ec));
        assert_eq!(ec.ip, 0x1);
        assert_eq!(ec.sp, 0x100);
        assert_eq!(ec.sl, 0x90);
              
        println!("{}",ec);
        println!("{}",ec.mapping_unit.msr);

    }

    #[test]
    fn rtfni_test()  {
        let mut ec = ExecutionContext::new();

        ec.load_initial_memory(vec![0;30]);

        ec.fp = 0x30;
        ec.sp = 0x30;
        ec.sl = 0x20;
        ec.ip = 0;

        ec.push_a_single_word_to_the_stack(0x10); // SL
        ec.push_a_single_word_to_the_stack(0x20); // SP
        ec.push_a_single_word_to_the_stack(0xE002); // MSR
        ec.push_a_single_word_to_the_stack(0x10); // Return adr
        ec.push_a_single_word_to_the_stack(0x34); // CMASK

        ec.mapping_unit.add_single_mapping_for_test_purpose(2,0,0);

        explain("RTFNI",Some(&mut ec));
        assert_eq!(ec.ip, 0x10);
        assert_eq!(ec.sp, 0x20);
        assert_eq!(ec.sl, 0x10);
        //assert_eq!(ec.sl, 0x10); // MSR
        assert_eq!(ec.mapping_unit.read_word_from_memory(5,true), 0x34);

        println!("{}",ec);

    }


    }

