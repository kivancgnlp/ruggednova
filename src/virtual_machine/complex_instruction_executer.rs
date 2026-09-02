// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::virtual_machine::ExecutionContext;

#[derive(Debug)]
struct DequeInfo {
    capacity:u16,
    occupancy:u16,
    current_top_element_index:u16,
    next_bottom_element_index:u16,
}

const DEQUE_CAPACITY_OFFSET:u16 = 0;
const DEQUE_OCCUPANCY_OFFSET:u16 = 1;
const CURRENT_TOP_ELEMENT_OFFSET:u16 = 2;
const NEXT_BOTTOM_ELEMENT_OFFSET:u16= 3;



fn read_deque_header(ec: &mut ExecutionContext) -> DequeInfo {

    let info_header_base_adr = ec.ac[2];

    let capacity = ec.mapping_unit.read_word_from_memory(info_header_base_adr + DEQUE_CAPACITY_OFFSET,true);
    let occupancy = ec.mapping_unit.read_word_from_memory(info_header_base_adr + DEQUE_OCCUPANCY_OFFSET,true);
    let current_top_element_index = ec.mapping_unit.read_word_from_memory(info_header_base_adr + CURRENT_TOP_ELEMENT_OFFSET,true);
    let next_bottom_element_index = ec.mapping_unit.read_word_from_memory(info_header_base_adr + NEXT_BOTTOM_ELEMENT_OFFSET,true);

    DequeInfo {capacity, occupancy, current_top_element_index, next_bottom_element_index }

}

fn update_deque_header(ec: &mut ExecutionContext, deque_info: DequeInfo)  {

    let info_header_base_adr = ec.ac[2];

    ec.mapping_unit.write_word_to_memory(info_header_base_adr + DEQUE_CAPACITY_OFFSET,deque_info.capacity,true);
    ec.mapping_unit.write_word_to_memory(info_header_base_adr + DEQUE_OCCUPANCY_OFFSET,deque_info.occupancy,true);
    ec.mapping_unit.write_word_to_memory(info_header_base_adr + CURRENT_TOP_ELEMENT_OFFSET, deque_info.current_top_element_index, true);
    ec.mapping_unit.write_word_to_memory(info_header_base_adr + NEXT_BOTTOM_ELEMENT_OFFSET, deque_info.next_bottom_element_index, true);

}

fn dump_deque_header(ec: &mut ExecutionContext) {
    let q_info = read_deque_header(ec);
    println!("Q info : {:?}",q_info);
}

fn decrement_by_checking_capacity_bounds(index: &mut u16, capacity: u16) {

    if *index > 0 {
        *index -= 1;
    }else {
        *index = capacity - 1;
    }
}

fn increment_by_checking_capacity_bounds(index: &mut u16, capacity: u16) {

    if *index < capacity - 1 {
        *index += 1;
    }else {
        *index = 0;
    }
}
pub(crate) fn deque_remove_from_bottom(ec: &mut ExecutionContext) -> Option<u16>{

    let mut deque_info = read_deque_header(ec);

    if deque_info.occupancy == 0 {
        return None;
    }

    let info_header_base_adr = ec.ac[2];

    decrement_by_checking_capacity_bounds(&mut deque_info.next_bottom_element_index,deque_info.capacity);
    let read_element = ec.mapping_unit.read_word_from_memory(info_header_base_adr + 4 + deque_info.next_bottom_element_index, true);


    deque_info.occupancy -= 1;


    update_deque_header(ec, deque_info);
    Some(read_element)
}

pub(crate) fn deque_add_to_bottom(ec: &mut ExecutionContext) -> bool{

    let mut deque_info = read_deque_header(ec);

    if deque_info.occupancy == deque_info.capacity {
        return false;
    }

    let info_header_base_adr = ec.ac[2];
    let element_to_write = ec.ac[0];

    ec.mapping_unit.write_word_to_memory(info_header_base_adr + 4 + deque_info.next_bottom_element_index, element_to_write, true);
    increment_by_checking_capacity_bounds(&mut deque_info.next_bottom_element_index, deque_info.capacity);




    deque_info.occupancy += 1;


    update_deque_header(ec, deque_info);
    true

}


pub(crate) fn deque_add_to_top(ec: &mut ExecutionContext) -> bool{

    let mut deque_info = read_deque_header(ec);

    if deque_info.occupancy == deque_info.capacity {
        return false;
    }

    let info_header_base_adr = ec.ac[2];
    let element_to_write = ec.ac[0];

    decrement_by_checking_capacity_bounds(&mut deque_info.current_top_element_index, deque_info.capacity);
    ec.mapping_unit.write_word_to_memory(info_header_base_adr + 4 + deque_info.current_top_element_index, element_to_write, true);

    deque_info.occupancy += 1;


    update_deque_header(ec, deque_info);
    true
}

pub(crate) fn deque_remove_from_top(ec: &mut ExecutionContext) -> Option<u16>{

    let mut deque_info = read_deque_header(ec);

    if deque_info.occupancy == 0 {
        return None;
    }

    let info_header_base_adr = ec.ac[2];

    let read_element = ec.mapping_unit.read_word_from_memory(info_header_base_adr + 4 + deque_info.current_top_element_index, true);
    increment_by_checking_capacity_bounds(&mut deque_info.current_top_element_index,deque_info.capacity);

    deque_info.occupancy -= 1;


    update_deque_header(ec, deque_info);
    Some(read_element)
}

pub(crate) fn word_seach_fs(ec: &mut ExecutionContext, mask:u16) -> Option<u16> {

    let lower_limit_search_value = ec.ac[0];
    let upper_limit_search_value = ec.ac[1];

    let search_adr_base = ec.ac[2] + 1;
    let search_adr_last = ec.ac[3] ;

    for adr in search_adr_base..search_adr_last {
        let val = ec.mapping_unit.read_word_from_memory(adr,true);
        let val = val & mask;

        if val >= lower_limit_search_value && val <= upper_limit_search_value {
            return Some(adr);
        }
    }

    None

}

pub(crate) fn move_byte_string_with_terminator(ec : &mut ExecutionContext) {

    
    let term_char = (ec.ac[0] & 0xff) as u8;
    
    loop{
        if ec.ac[1] == 0 {
            break;
        }

        let (src_word_adr,src_second_byte) = crate::instruction_decoder::generic_instruction_format_decoder::generic_format_with_two_acc_for_byte_ops_wo_extra_word::form_abn_byte_adr(2, ec);
        let (dst_word_adr,dst_second_byte) = crate::instruction_decoder::generic_instruction_format_decoder::generic_format_with_two_acc_for_byte_ops_wo_extra_word::form_abn_byte_adr(3, ec);

        let read_byte = ec.load_byte_from_mem(src_word_adr, src_second_byte);
        ec.store_byte_to_mem(dst_word_adr,dst_second_byte,read_byte);
         
        ec.ac[2] += 1;
        ec.ac[3] += 1;
        ec.ac[1] -= 1;

        if read_byte == term_char {
            break;
        }
    }
    
}

pub(crate) fn move_byte_string(ec : &mut ExecutionContext) {

    loop{
        if ec.ac[1] == 0 {
            break;
        }

        let (src_word_adr,src_second_byte) = crate::instruction_decoder::generic_instruction_format_decoder::generic_format_with_two_acc_for_byte_ops_wo_extra_word::form_abn_byte_adr(2, ec);
        let (dst_word_adr,dst_second_byte) = crate::instruction_decoder::generic_instruction_format_decoder::generic_format_with_two_acc_for_byte_ops_wo_extra_word::form_abn_byte_adr(3, ec);

        let read_byte = ec.load_byte_from_mem(src_word_adr, src_second_byte);
        ec.store_byte_to_mem(dst_word_adr,dst_second_byte,read_byte);

        ec.ac[2] += 1;
        ec.ac[3] += 1;
        ec.ac[1] -= 1;


    }


}


#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn deque_test_01(){ 

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.write_word_to_memory(10,5,true);
        ec.ac[2] = 10;

        ec.ac[0] = 10; deque_add_to_bottom(&mut ec); dump_deque_header(&mut ec);
        ec.ac[0] = 20; deque_add_to_top(&mut ec); dump_deque_header(&mut ec);
        ec.ac[0] = 30; deque_add_to_top(&mut ec); dump_deque_header(&mut ec);
        ec.ac[0] = 40; deque_add_to_bottom(&mut ec); dump_deque_header(&mut ec);
        ec.ac[0] = 50; deque_add_to_bottom(&mut ec); dump_deque_header(&mut ec);

        let e = deque_remove_from_top(&mut ec).unwrap();
        println!("Element : {}",e);
        assert_eq!(e, 30);

        let e = deque_remove_from_top(&mut ec).unwrap();
        println!("Element : {}",e);
        assert_eq!(e, 20);

        let e = deque_remove_from_top(&mut ec).unwrap();
        println!("Element : {}",e);
        assert_eq!(e, 10);

        ec.ac[0] = 60; deque_add_to_top(&mut ec); dump_deque_header(&mut ec);


        println!("End of test 01");
    }

    #[test]
    fn deque_test_02(){

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.write_word_to_memory(10,5,true);
        ec.ac[2] = 10;

        for i in 1..10 {
            ec.ac[0] = i; deque_add_to_bottom(&mut ec); dump_deque_header(&mut ec);
            let e = deque_remove_from_top(&mut ec).unwrap();
            println!("Element : {}",e);
            assert_eq!(e, i);
        }

        println!("End of test 02");
    }


    #[test]
    fn deque_test_03(){

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.write_word_to_memory(10,5,true);
        ec.ac[2] = 10;

        for i in 1..10 {
            ec.ac[0] = i; deque_add_to_top(&mut ec); dump_deque_header(&mut ec);
            let e = deque_remove_from_bottom(&mut ec).unwrap();
            println!("Element : {}",e);
            assert_eq!(e, i);
        }

        println!("End of test 03");
    }

    #[test]
    fn deque_test_04(){

        let mut ec = ExecutionContext::new();

        ec.mapping_unit.write_word_to_memory(10,5,true);
        ec.ac[2] = 10;

        for i in 1..6 {
            ec.ac[0] = i; deque_add_to_bottom(&mut ec); dump_deque_header(&mut ec);
    
        }

        for i in 1..6 {            
            let e = deque_remove_from_bottom(&mut ec).unwrap();
            println!("Element : {}",e);
            assert_eq!(e, 6-i);
        }

        println!("End of test 04");
    }
}

