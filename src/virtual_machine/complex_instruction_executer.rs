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



/// The storage area begins four words past the header: manual p. 3-24, "set ((AC2) + ((AC2) + 3)
/// + 4) = (AC0)". Wrapping, because a zero-length deque can leave a slot pointer at 0xffff.
fn slot_address(header_base: u16, slot_index: u16) -> u16 {
    header_base.wrapping_add(4).wrapping_add(slot_index)
}

fn read_deque_header(ec: &mut ExecutionContext) -> DequeInfo {

    let info_header_base_adr = ec.ac[2];

    let capacity = ec.mapping_unit.read_word_from_memory(info_header_base_adr.wrapping_add(DEQUE_CAPACITY_OFFSET),true);
    let occupancy = ec.mapping_unit.read_word_from_memory(info_header_base_adr.wrapping_add(DEQUE_OCCUPANCY_OFFSET),true);
    let current_top_element_index = ec.mapping_unit.read_word_from_memory(info_header_base_adr.wrapping_add(CURRENT_TOP_ELEMENT_OFFSET),true);
    let next_bottom_element_index = ec.mapping_unit.read_word_from_memory(info_header_base_adr.wrapping_add(NEXT_BOTTOM_ELEMENT_OFFSET),true);

    DequeInfo {capacity, occupancy, current_top_element_index, next_bottom_element_index }

}

fn update_deque_header(ec: &mut ExecutionContext, deque_info: DequeInfo)  {

    let info_header_base_adr = ec.ac[2];

    ec.mapping_unit.write_word_to_memory(info_header_base_adr.wrapping_add(DEQUE_CAPACITY_OFFSET),deque_info.capacity,true);
    ec.mapping_unit.write_word_to_memory(info_header_base_adr.wrapping_add(DEQUE_OCCUPANCY_OFFSET),deque_info.occupancy,true);
    ec.mapping_unit.write_word_to_memory(info_header_base_adr.wrapping_add(CURRENT_TOP_ELEMENT_OFFSET), deque_info.current_top_element_index, true);
    ec.mapping_unit.write_word_to_memory(info_header_base_adr.wrapping_add(NEXT_BOTTOM_ELEMENT_OFFSET), deque_info.next_bottom_element_index, true);

}

fn dump_deque_header(ec: &mut ExecutionContext) {
    let q_info = read_deque_header(ec);
    println!("Q info : {:?}",q_info);
}

/// Step a deque slot pointer backwards, wrapping at the top of the circular buffer.
///
/// Manual p. 3-24 (ATD) and p. 3-25 (RBD) both state it the same way:
/// "If ((AC2) + 2) = 0, set ((AC2) + 2) = ((AC2)). Decrement ((AC2) + 2) by 1."
///
/// Written exactly as the manual has it rather than as `index = capacity - 1`, which is
/// equivalent for a deque of non-zero length but underflows when the length is zero. INS64's
/// IDEA at 011202 drives all four deque instructions at a zero-length deque on purpose, and
/// that underflow is what used to panic the run.
fn retreat_slot_pointer(index: &mut u16, capacity: u16) {

    if *index == 0 {
        *index = capacity;
    }
    *index = index.wrapping_sub(1);
}

/// Step a deque slot pointer forwards, wrapping at the end of the circular buffer.
///
/// Manual p. 3-24 (ABD): "Increment ((AC2) + 3) by 1. Then if ((AC2) + 3) = ((AC2)), set
/// ((AC2) + 3) = 0." RTD on p. 3-26 says the same for the top pointer.
///
/// Note the wrap test is an equality against the LENGTH after incrementing, not a comparison
/// against length - 1 before it. Same result for a well-formed deque, no underflow for a
/// degenerate one.
fn advance_slot_pointer(index: &mut u16, capacity: u16) {

    *index = index.wrapping_add(1);
    if *index == capacity {
        *index = 0;
    }
}
pub(crate) fn deque_remove_from_bottom(ec: &mut ExecutionContext) -> Option<u16>{

    let mut deque_info = read_deque_header(ec);

    if deque_info.occupancy == 0 {
        return None;
    }

    let info_header_base_adr = ec.ac[2];

    retreat_slot_pointer(&mut deque_info.next_bottom_element_index,deque_info.capacity);
    let read_element = ec.mapping_unit.read_word_from_memory(slot_address(info_header_base_adr, deque_info.next_bottom_element_index), true);


    deque_info.occupancy = deque_info.occupancy.wrapping_sub(1);


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

    ec.mapping_unit.write_word_to_memory(slot_address(info_header_base_adr, deque_info.next_bottom_element_index), element_to_write, true);
    advance_slot_pointer(&mut deque_info.next_bottom_element_index, deque_info.capacity);




    deque_info.occupancy = deque_info.occupancy.wrapping_add(1);


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

    retreat_slot_pointer(&mut deque_info.current_top_element_index, deque_info.capacity);
    ec.mapping_unit.write_word_to_memory(slot_address(info_header_base_adr, deque_info.current_top_element_index), element_to_write, true);

    deque_info.occupancy = deque_info.occupancy.wrapping_add(1);


    update_deque_header(ec, deque_info);
    true
}

pub(crate) fn deque_remove_from_top(ec: &mut ExecutionContext) -> Option<u16>{

    let mut deque_info = read_deque_header(ec);

    if deque_info.occupancy == 0 {
        return None;
    }

    let info_header_base_adr = ec.ac[2];

    let read_element = ec.mapping_unit.read_word_from_memory(slot_address(info_header_base_adr, deque_info.current_top_element_index), true);
    advance_slot_pointer(&mut deque_info.current_top_element_index,deque_info.capacity);

    deque_info.occupancy = deque_info.occupancy.wrapping_sub(1);


    update_deque_header(ec, deque_info);
    Some(read_element)
}

pub(crate) fn word_seach_fs(ec: &mut ExecutionContext, mask:u16) -> Option<u16> {

    let lower_limit_search_value = ec.ac[0];
    let upper_limit_search_value = ec.ac[1];

    // Manual p. 3-19: "Search a file whose first address is (AC2) + 1 and whose last address is
    // (AC3)." (AC3) is the last address of the file, not one past it, so the range is inclusive.
    let search_adr_base = ec.ac[2].wrapping_add(1);
    let search_adr_last = ec.ac[3] ;

    if search_adr_base > search_adr_last {
        return None;
    }

    for adr in search_adr_base..=search_adr_last {
        let val = ec.mapping_unit.read_word_from_memory(adr,true);
        let val = val & mask;

        if val >= lower_limit_search_value && val <= upper_limit_search_value {
            return Some(adr);
        }
    }

    None

}

/// LKLS is three words long, so "not found" is simply the next instruction and "found" skips one.
pub(crate) const LKLS_EXIT_NOT_FOUND: u16 = 3;
pub(crate) const LKLS_EXIT_FOUND: u16     = 4;

/// LINKED LIST SEARCH, manual pp. 3-20..3-21. A three-word instruction:
///
/// ```text
///   077200     LKLS
///   <MASK>     at PC+1
///   <OFFSET>   at PC+2
/// ```
///
/// The assembler writes those as three separate lines; the manual says so explicitly.
///
///   AC0  lower limit of search value
///   AC1  upper limit of search value
///   AC2  address of the link word of the first block in the list
///   AC3  receives the address of the link entry one prior to the matching link
///
/// The manual's own step:
///
/// > let result = ((PC) + 1) logically ANDed with ((AC2 + ((PC) + 2))). If (AC0) <= result <=
/// > (AC1), increment (PC) by 4 and fetch the next instruction from that location. If ((AC2)) !=
/// > 177777, let (AC3) = (AC2), let (AC2) = ((AC2)) and loop to the beginning of this description.
///
/// Three things that follow from the order of those sentences:
///
///   * The range test runs BEFORE the link is followed, so the first block is tested with AC2 and
///     AC3 exactly as the caller left them.
///   * `177777` in the link word marks the end of the list, and it is only consulted after the
///     range test has already failed — so a match in the last block is still found.
///   * AC3 is written only when a link is actually followed. On a match in the very first block it
///     keeps whatever the caller put there, which is why the manual's note says to preload it:
///     "LEF 3, pointer to first link / LDA 2, 0, 3".
///
/// The OFFSET is signed — "may be located positive or negative to the link address word" — so the
/// checked word is reached by a wrapping add.
///
/// A list that links back on itself loops here forever, exactly as the hardware would; the manual
/// offers no depth limit and relies on the instruction being interruptible.
pub(crate) fn linked_list_search(ec: &mut ExecutionContext) -> u16 {

    const END_OF_LIST: u16 = 0o177777;

    let mask   = ec.read_word_from_mem_using_instruction_map_pc_relative(1, false);
    let offset = ec.read_word_from_mem_using_instruction_map_pc_relative(2, false);

    let lower = ec.ac[0];
    let upper = ec.ac[1];

    loop {
        let checked_address = ec.ac[2].wrapping_add(offset);
        let result = mask & ec.mapping_unit.read_word_from_memory(checked_address, true);

        if result >= lower && result <= upper {
            return LKLS_EXIT_FOUND;
        }

        let link = ec.mapping_unit.read_word_from_memory(ec.ac[2], true);
        if link == END_OF_LIST {
            // AC2 is left on the last block, as "Register Contents on Exit" requires.
            return LKLS_EXIT_NOT_FOUND;
        }

        ec.ac[3] = ec.ac[2];
        ec.ac[2] = link;
    }
}

/// How many words the PC advances past a byte-string instruction. All four of COMB, COMBT, SRCB
/// and SRCBT are multi-way: they report their result by choosing an exit rather than by setting a
/// flag. See the "Summary of Exits" table on each instruction page.
pub(crate) type ByteStringExit = u16;

pub(crate) const EXIT_PC_PLUS_1: ByteStringExit = 1;
pub(crate) const EXIT_PC_PLUS_2: ByteStringExit = 2;
pub(crate) const EXIT_PC_PLUS_3: ByteStringExit = 3;

/// Advance a byte address register by one byte.
///
/// Manual p. 3-9: "A byte address is the sum of (bn) one bit left shifted and (ac)", and "The IBA
/// instruction adds 1 to (acn). If carry out of (acn) occurs, bit 0 of (bn) is complemented."
/// Section 2.19 applies the same rule to any byte-address increment, so a string that runs past
/// the end of the accumulator carries into the base register instead of silently wrapping to the
/// start of the same 64K.
///
/// `n` is 2 or 3, selecting the AC2/BR2 or AC3/BR3 pair.
fn increment_byte_address(ec: &mut ExecutionContext, n: usize) {
    let (incremented, carry) = ec.ac[n].overflowing_add(1);
    ec.ac[n] = incremented;
    if carry {
        ec.br[n - 2] ^= 0x8000;
    }
}

fn fetch_byte(ec: &mut ExecutionContext, n: u16) -> u8 {
    let (word_adr, second_byte) =
        crate::instruction_decoder::generic_instruction_format_decoder::generic_format_with_two_acc_for_byte_ops_wo_extra_word::form_abn_byte_adr(n, ec);
    ec.load_byte_from_mem(word_adr, second_byte)
}

/// COMPARE BYTE STRINGS, manual p. 3-9. AC1 = byte count, AB2 = string 1, AB3 = string 2.
///
/// Exits: PC+1 equal, PC+2 string 1 < string 2, PC+3 string 1 > string 2.
///
/// Two things the instruction page leaves implicit, both settled by its own "Register Contents on
/// Exit" table:
///
///   * The loop text says only "Increment (AB2) and (AB3) by 1, and loop back" — it never mentions
///     decrementing AC1, which would loop forever. COMBT on the next page spells the decrement
///     out, and "AC1: Number of bytes remaining in strings after the last byte pair tested" only
///     makes sense with it. So AC1 counts down.
///   * The mismatch exits mention only the PC, but "AB2: Address of next byte in String 1 after
///     the last byte tested" means the pointers advance past the mismatching pair too, exactly as
///     COMBT states explicitly.
pub(crate) fn compare_byte_strings(ec: &mut ExecutionContext) -> ByteStringExit {

    loop {
        if ec.ac[1] == 0 {
            return EXIT_PC_PLUS_1;           // zero byte count: the strings are equal
        }

        let b2 = fetch_byte(ec, 2);
        let b3 = fetch_byte(ec, 3);

        increment_byte_address(ec, 2);
        increment_byte_address(ec, 3);
        ec.ac[1] = ec.ac[1].wrapping_sub(1);

        if b2 < b3 {
            return EXIT_PC_PLUS_2;
        }
        if b2 > b3 {
            return EXIT_PC_PLUS_3;
        }
    }
}

/// COMPARE BYTE STRINGS WITH TERMINATOR, manual p. 3-10. As COMB, plus AC0 bits 8-15 hold the
/// terminator character.
///
/// The manual's order of tests matters and is preserved here: equal-and-terminator ends the
/// comparison as equal, equal-and-not-terminator continues, and only then do the terminator and
/// magnitude tests decide which string is smaller. A terminator in string 1 makes it the shorter,
/// hence the lesser, string.
pub(crate) fn compare_byte_strings_with_terminator(ec: &mut ExecutionContext) -> ByteStringExit {

    let terminator = (ec.ac[0] & 0xff) as u8;

    loop {
        if ec.ac[1] == 0 {
            return EXIT_PC_PLUS_1;
        }

        let b2 = fetch_byte(ec, 2);
        let b3 = fetch_byte(ec, 3);

        increment_byte_address(ec, 2);
        increment_byte_address(ec, 3);
        ec.ac[1] = ec.ac[1].wrapping_sub(1);

        if b2 == b3 {
            if b2 == terminator {
                return EXIT_PC_PLUS_1;       // both strings ended together
            }
            continue;                        // equal so far, keep going
        }

        if b2 == terminator || b2 < b3 {
            return EXIT_PC_PLUS_2;
        }

        // b3 == terminator, or b2 > b3
        return EXIT_PC_PLUS_3;
    }
}

/// SEARCH BYTE STRING, manual p. 3-14. AC0 bits 0-7 hold the search character, AC1 the byte count,
/// AB2 the string.
///
/// Exits: PC+1 not found, PC+2 found. AC0 is unchanged; AC1 and AB2 are left pointing just past
/// the byte that ended the search.
pub(crate) fn search_byte_string(ec: &mut ExecutionContext) -> ByteStringExit {

    let search_character = (ec.ac[0] >> 8) as u8;

    loop {
        if ec.ac[1] == 0 {
            return EXIT_PC_PLUS_1;           // not found
        }

        let byte = fetch_byte(ec, 2);

        increment_byte_address(ec, 2);
        ec.ac[1] = ec.ac[1].wrapping_sub(1);

        if byte == search_character {
            return EXIT_PC_PLUS_2;           // found
        }
    }
}

/// SEARCH BYTE STRING WITH TERMINATOR, manual p. 3-15. As SRCB, plus AC0 bits 8-15 hold the
/// terminator character.
///
/// Note both "not found" and "terminator reached" take the SAME exit, PC+1 — the summary of exits
/// lists them together. The caller distinguishes them by looking at AC1.
pub(crate) fn search_byte_string_with_terminator(ec: &mut ExecutionContext) -> ByteStringExit {

    let search_character = (ec.ac[0] >> 8) as u8;
    let terminator       = (ec.ac[0] & 0xff) as u8;

    loop {
        if ec.ac[1] == 0 {
            return EXIT_PC_PLUS_1;
        }

        let byte = fetch_byte(ec, 2);

        increment_byte_address(ec, 2);
        ec.ac[1] = ec.ac[1].wrapping_sub(1);

        if byte == search_character {
            return EXIT_PC_PLUS_2;
        }
        if byte == terminator {
            return EXIT_PC_PLUS_1;
        }
    }
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
         
        // Section 2.19 / the IBA rule on p. 3-9: a carry out of (acn) complements bit 0 of (bn),
        // so a byte string running past the end of the accumulator carries into the base register
        // instead of wrapping to the start of the same 64K. These were bare `+= 1` before, which
        // also panicked in debug builds on the wrap.
        increment_byte_address(ec, 2);
        increment_byte_address(ec, 3);
        ec.ac[1] = ec.ac[1].wrapping_sub(1);

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

        increment_byte_address(ec, 2);
        increment_byte_address(ec, 3);
        ec.ac[1] = ec.ac[1].wrapping_sub(1);


    }


}


#[cfg(test)]
mod tests {

    use super::*;

    /// Manual p. 3-19: "(AC2) + 1" is the first address and "(AC3)" is the LAST address of the
    /// file, so a match in the final word must be found.
    #[test]
    fn fs_searches_the_last_word_of_the_file(){
        let mut ec = ExecutionContext::new();
        let mut mem = vec![0u16; 32];
        mem[10] = 0x1111;   // first word of the file
        mem[11] = 0x2222;
        mem[12] = 0x3333;   // last word of the file, and the only match
        ec.load_initial_memory(mem);

        ec.ac[0] = 0x3333;  // lower limit
        ec.ac[1] = 0x3333;  // upper limit
        ec.ac[2] = 9;       // first address is AC2 + 1 == 10
        ec.ac[3] = 12;      // last address, inclusive

        assert_eq!(word_seach_fs(&mut ec, 0xFFFF), Some(12));
    }

    #[test]
    fn fs_reports_no_match_without_running_off_the_end(){
        let mut ec = ExecutionContext::new();
        let mut mem = vec![0u16; 32];
        mem[10] = 1; mem[11] = 2; mem[12] = 3;
        mem[13] = 0x3333;   // just past the file — must NOT be examined
        ec.load_initial_memory(mem);

        ec.ac[0] = 0x3333; ec.ac[1] = 0x3333;
        ec.ac[2] = 9; ec.ac[3] = 12;

        assert_eq!(word_seach_fs(&mut ec, 0xFFFF), None);
    }

    /// An empty file (AC3 <= AC2) must terminate rather than wrap all the way round.
    #[test]
    fn fs_handles_an_empty_file(){
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 32]);
        ec.ac[0] = 0; ec.ac[1] = 0xFFFF;
        ec.ac[2] = 12; ec.ac[3] = 12;     // first address 13 > last address 12
        assert_eq!(word_seach_fs(&mut ec, 0xFFFF), None);
    }

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


#[cfg(test)]
mod byte_string_instructions {
    use super::*;

    /// Lay out a byte string at word address `at`. Bytes pack big-endian: byte 0 of a word is the
    /// high half, byte 1 the low half.
    fn put_bytes(ec: &mut ExecutionContext, at: u16, bytes: &[u8]) {
        for (i, pair) in bytes.chunks(2).enumerate() {
            let hi = pair[0] as u16;
            let lo = *pair.get(1).unwrap_or(&0) as u16;
            ec.mapping_unit.write_word_to_memory(at + i as u16, (hi << 8) | lo, true);
        }
    }

    /// AB2 = (BR2 << 1) + AC2, so with the base registers at zero the accumulator IS the byte
    /// address and word `w` starts at byte `2w`.
    fn ctx() -> ExecutionContext {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 0x100]);
        ec
    }

    // ---- COMB, manual p. 3-9 -------------------------------------------------------------

    #[test]
    fn comb_equal_strings_exit_pc_plus_1() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"ABCD");
        put_bytes(&mut ec, 0x20, b"ABCD");
        ec.ac[1] = 4; ec.ac[2] = 0x20; ec.ac[3] = 0x40;   // byte addresses of words 0x10 / 0x20

        assert_eq!(compare_byte_strings(&mut ec), EXIT_PC_PLUS_1);
        assert_eq!(ec.ac[1], 0, "the whole count was consumed");
        assert_eq!(ec.ac[2], 0x24, "AB2 past the last byte tested");
        assert_eq!(ec.ac[3], 0x44);
    }

    /// The instruction page never says the loop decrements AC1 — but "Register Contents on Exit:
    /// AC1 Number of bytes remaining in strings after the last byte pair tested" requires it, and
    /// without it the loop could not terminate. COMBT on the next page spells it out.
    #[test]
    fn comb_stops_at_the_first_mismatch_and_reports_which_is_smaller() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"ABXD");
        put_bytes(&mut ec, 0x20, b"ABYD");
        ec.ac[1] = 4; ec.ac[2] = 0x20; ec.ac[3] = 0x40;

        assert_eq!(compare_byte_strings(&mut ec), EXIT_PC_PLUS_2, "'X' < 'Y'");
        assert_eq!(ec.ac[1], 1, "one pair left untested after the third");
        assert_eq!(ec.ac[2], 0x23, "pointers advance past the mismatching pair too");
        assert_eq!(ec.ac[3], 0x43);
    }

    #[test]
    fn comb_reports_the_greater_string() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"AZ");
        put_bytes(&mut ec, 0x20, b"AB");
        ec.ac[1] = 2; ec.ac[2] = 0x20; ec.ac[3] = 0x40;

        assert_eq!(compare_byte_strings(&mut ec), EXIT_PC_PLUS_3);
    }

    /// "If (AC1) = 0, the byte count is zero and the strings are equal."
    #[test]
    fn comb_with_a_zero_count_is_equal_and_touches_nothing() {
        let mut ec = ctx();
        ec.ac[1] = 0; ec.ac[2] = 0x20; ec.ac[3] = 0x40;

        assert_eq!(compare_byte_strings(&mut ec), EXIT_PC_PLUS_1);
        assert_eq!(ec.ac[2], 0x20);
        assert_eq!(ec.ac[3], 0x40);
    }

    // ---- COMBT, manual p. 3-10 ----------------------------------------------------------

    /// "If ((AB2)) = ((AB3)) = Terminator Character, the strings are equal."
    #[test]
    fn combt_both_strings_ending_together_is_equal() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"AB\0\0");
        put_bytes(&mut ec, 0x20, b"AB\0\0");
        ec.ac[0] = 0x0000;                                // terminator = 0 in the low byte
        ec.ac[1] = 4; ec.ac[2] = 0x20; ec.ac[3] = 0x40;

        assert_eq!(compare_byte_strings_with_terminator(&mut ec), EXIT_PC_PLUS_1);
        assert_eq!(ec.ac[1], 1, "stopped on the terminator, one byte still uncounted");
    }

    /// "If ((AB2)) = Terminator Character ... then string 1 is < string 2." A string that ends
    /// first is the shorter, hence the lesser one — even though its terminator byte may compare
    /// as greater.
    #[test]
    fn combt_the_string_that_terminates_first_is_the_lesser() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"AB\xff\xff");           // terminator 0xff right after "AB"
        put_bytes(&mut ec, 0x20, b"ABCD");
        ec.ac[0] = 0x00ff;
        ec.ac[1] = 4; ec.ac[2] = 0x20; ec.ac[3] = 0x40;

        assert_eq!(compare_byte_strings_with_terminator(&mut ec), EXIT_PC_PLUS_2,
                   "string 1 ended first, so it is the lesser -- 0xff > 'C' must not decide it");
    }

    // ---- SRCB / SRCBT, manual pp. 3-14, 3-15 ---------------------------------------------

    /// AC0 bits 0-7 hold the search character.
    #[test]
    fn srcb_finds_the_character_and_leaves_the_pointer_past_it() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"ABCD");
        ec.ac[0] = (b'C' as u16) << 8;
        ec.ac[1] = 4; ec.ac[2] = 0x20;

        assert_eq!(search_byte_string(&mut ec), EXIT_PC_PLUS_2);
        assert_eq!(ec.ac[2], 0x23, "next byte address after the search character");
        assert_eq!(ec.ac[1], 1, "bytes remaining after it");
        assert_eq!(ec.ac[0], (b'C' as u16) << 8, "AC0 unchanged");
    }

    #[test]
    fn srcb_not_found_exhausts_the_count() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"ABCD");
        ec.ac[0] = (b'Z' as u16) << 8;
        ec.ac[1] = 4; ec.ac[2] = 0x20;

        assert_eq!(search_byte_string(&mut ec), EXIT_PC_PLUS_1);
        assert_eq!(ec.ac[1], 0);
        assert_eq!(ec.ac[2], 0x24);
    }

    /// Both "not found" and "terminator reached" take the PC+1 exit; the caller tells them apart
    /// by AC1, which is non-zero only in the terminator case.
    #[test]
    fn srcbt_stops_on_the_terminator_by_the_same_exit_as_not_found() {
        let mut ec = ctx();
        put_bytes(&mut ec, 0x10, b"AB\0D");
        ec.ac[0] = ((b'Z' as u16) << 8) | 0x00;           // search 'Z', terminator 0
        ec.ac[1] = 4; ec.ac[2] = 0x20;

        assert_eq!(search_byte_string_with_terminator(&mut ec), EXIT_PC_PLUS_1);
        assert_eq!(ec.ac[1], 1, "stopped early on the terminator, unlike a plain miss");
    }

    /// Manual p. 3-9, the IBA rule: "If carry out of (acn) occurs, bit 0 of (bn) is complemented."
    /// A byte string that runs past the end of the accumulator carries into the base register
    /// rather than wrapping to the start of the same 64K.
    #[test]
    fn a_byte_address_carries_out_of_the_accumulator_into_the_base_register() {
        let mut ec = ctx();
        ec.ac[2] = 0xffff;
        ec.br[0] = 0x0000;

        increment_byte_address(&mut ec, 2);

        assert_eq!(ec.ac[2], 0x0000);
        assert_eq!(ec.br[0], 0x8000, "bit 0 of BR2 complemented");

        // and complemented again on the next carry, not merely set
        ec.ac[2] = 0xffff;
        increment_byte_address(&mut ec, 2);
        assert_eq!(ec.br[0], 0x0000);
    }
}

#[cfg(test)]
mod deque_slot_pointers {
    use super::*;

    /// Manual p. 3-24 (ABD) and p. 3-26 (RTD): "Increment ((AC2) + 3) by 1. Then if ((AC2) + 3) =
    /// ((AC2)), set ((AC2) + 3) = 0." The wrap is an equality against the LENGTH after the
    /// increment, not a comparison against length - 1 before it.
    #[test]
    fn advancing_wraps_at_the_end_of_the_buffer() {
        let mut i = 0;
        for expected in [1, 2, 3, 4, 0, 1] {
            advance_slot_pointer(&mut i, 5);
            assert_eq!(i, expected);
        }
    }

    /// Manual p. 3-24 (ATD) and p. 3-25 (RBD): "If ((AC2) + 2) = 0, set ((AC2) + 2) = ((AC2)).
    /// Decrement ((AC2) + 2) by 1."
    #[test]
    fn retreating_wraps_at_the_start_of_the_buffer() {
        let mut i = 2;
        for expected in [1, 0, 4, 3] {
            retreat_slot_pointer(&mut i, 5);
            assert_eq!(i, expected);
        }
    }

    /// A zero-length deque is the degenerate case INS64's IDEA at 011202 drives on purpose, and
    /// the old `capacity - 1` formulation panicked on it. Neither helper may underflow.
    #[test]
    fn a_zero_length_deque_does_not_underflow() {
        let mut i = 0;
        advance_slot_pointer(&mut i, 0);
        assert_eq!(i, 1, "nothing to wrap against");

        let mut i = 0;
        retreat_slot_pointer(&mut i, 0);
        assert_eq!(i, 0xffff, "wraps rather than panicking");
    }

    fn deque_at(ec: &mut ExecutionContext, base: u16, length: u16, in_use: u16, top: u16, bottom: u16) {
        ec.ac[2] = base;
        ec.mapping_unit.write_word_to_memory(base, length, true);
        ec.mapping_unit.write_word_to_memory(base + 1, in_use, true);
        ec.mapping_unit.write_word_to_memory(base + 2, top, true);
        ec.mapping_unit.write_word_to_memory(base + 3, bottom, true);
    }

    /// A properly initialised zero-length deque is both full and empty, so every one of the four
    /// instructions takes its error exit — which is exactly what IDEA's header comment says it
    /// expects: "ISSUES ALL FOUR DEQUE INSTRUCTIONS EXPECTING TO TAKE THE ERROR EXITS IN EACH
    /// CASE".
    #[test]
    fn every_operation_on_a_zero_length_deque_takes_the_error_exit() {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 0x40]);
        deque_at(&mut ec, 0x10, 0, 0, 0, 0);

        assert!(!deque_add_to_top(&mut ec), "full");
        assert!(!deque_add_to_bottom(&mut ec), "full");
        assert!(deque_remove_from_top(&mut ec).is_none(), "empty");
        assert!(deque_remove_from_bottom(&mut ec).is_none(), "empty");
    }

    /// And a deque whose header is inconsistent — length zero but slots claimed in use — must
    /// still not panic. This is the state the run actually reached before the fix.
    #[test]
    fn an_inconsistent_zero_length_header_does_not_panic() {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 0x40]);
        deque_at(&mut ec, 0x10, 0, 2, 4, 1);

        deque_add_to_bottom(&mut ec);
        deque_add_to_top(&mut ec);
        deque_remove_from_top(&mut ec);
        deque_remove_from_bottom(&mut ec);
    }
}

#[cfg(test)]
mod linked_list_search_tests {
    use super::*;

    /// Builds the list the manual's Figure 3-1 describes: blocks chained by a link word, with the
    /// word to be checked at a constant offset from that link word, and 177777 marking the end.
    ///
    /// ```text
    ///   0x20: link -> 0x30    0x21: 0x0005
    ///   0x30: link -> 0x40    0x31: 0x0010
    ///   0x40: link =  177777  0x41: 0x0020
    /// ```
    ///
    /// The opcode sits at 0x100, so MASK is at 0x101 and OFFSET at 0x102.
    fn list(mask: u16, offset: u16) -> ExecutionContext {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 0x200]);

        for (link_at, next, value) in [(0x20u16, 0x30u16, 0x0005u16),
                                       (0x30,    0x40,    0x0010),
                                       (0x40,    0o177777, 0x0020)] {
            ec.mapping_unit.write_word_to_memory(link_at, next, true);
            ec.mapping_unit.write_word_to_memory(link_at.wrapping_add(offset), value, true);
        }

        ec.ip = 0x100;
        ec.mapping_unit.write_word_to_memory(0x101, mask, true);
        ec.mapping_unit.write_word_to_memory(0x102, offset, true);
        ec
    }

    /// "If (AC0) <= result <= (AC1), increment (PC) by 4."
    #[test]
    fn a_match_in_a_later_block_reports_both_links() {
        let mut ec = list(0xffff, 1);
        ec.ac[0] = 0x0010; ec.ac[1] = 0x0010;
        ec.ac[2] = 0x20;                       // first link word
        ec.ac[3] = 0xdead;

        assert_eq!(linked_list_search(&mut ec), LKLS_EXIT_FOUND);
        assert_eq!(ec.ac[2], 0x30, "AC2: link word of the block that matched");
        assert_eq!(ec.ac[3], 0x20, "AC3: the link entry one prior");
        assert_eq!(ec.ac[0], 0x0010, "AC0 unchanged");
        assert_eq!(ec.ac[1], 0x0010, "AC1 unchanged");
    }

    /// The range test runs before any link is followed, so a match in the very first block leaves
    /// AC3 exactly as the caller set it — the reason the manual's note tells you to preload it.
    #[test]
    fn a_match_in_the_first_block_leaves_ac3_alone() {
        let mut ec = list(0xffff, 1);
        ec.ac[0] = 0x0005; ec.ac[1] = 0x0005;
        ec.ac[2] = 0x20;
        ec.ac[3] = 0xdead;

        assert_eq!(linked_list_search(&mut ec), LKLS_EXIT_FOUND);
        assert_eq!(ec.ac[2], 0x20);
        assert_eq!(ec.ac[3], 0xdead, "no link was followed, so AC3 is untouched");
    }

    /// The end-of-list word is consulted only after the range test has failed, so a match in the
    /// LAST block is still found rather than being cut off by the 177777 sentinel.
    #[test]
    fn a_match_in_the_last_block_is_still_found() {
        let mut ec = list(0xffff, 1);
        ec.ac[0] = 0x0020; ec.ac[1] = 0x0020;
        ec.ac[2] = 0x20;

        assert_eq!(linked_list_search(&mut ec), LKLS_EXIT_FOUND);
        assert_eq!(ec.ac[2], 0x40);
    }

    /// "AC2: Address of the link word of ... the last block if no matching entry is found."
    #[test]
    fn no_match_walks_to_the_end_of_the_list() {
        let mut ec = list(0xffff, 1);
        ec.ac[0] = 0x7000; ec.ac[1] = 0x7fff;
        ec.ac[2] = 0x20;

        assert_eq!(linked_list_search(&mut ec), LKLS_EXIT_NOT_FOUND);
        assert_eq!(ec.ac[2], 0x40, "left on the last block");
        assert_eq!(ec.ac[3], 0x30);
    }

    /// The search is against the MASKED word, so a mask can select a field within it.
    #[test]
    fn the_mask_is_applied_before_the_range_test() {
        let mut ec = list(0x00f0, 1);
        ec.ac[0] = 0x0010; ec.ac[1] = 0x0010;   // 0x0010 & 0x00f0 == 0x0010
        ec.ac[2] = 0x20;

        assert_eq!(linked_list_search(&mut ec), LKLS_EXIT_FOUND);
        assert_eq!(ec.ac[2], 0x30);

        // Without the mask 0x0020 would also be in range for [0x20,0x20]; with it, 0x0020 & 0x00f0
        // is 0x0020 as well -- so pick a limit the mask genuinely changes.
        let mut ec = list(0x000f, 1);
        ec.ac[0] = 0x0005; ec.ac[1] = 0x0005;   // 0x0005 & 0x000f matches block 1
        ec.ac[2] = 0x20;
        assert_eq!(linked_list_search(&mut ec), LKLS_EXIT_FOUND);
        assert_eq!(ec.ac[2], 0x20);
    }

    /// "The word to be checked ... may be located positive or negative to the link address word",
    /// so the offset is signed and reached by a wrapping add.
    #[test]
    fn the_offset_may_be_negative() {
        let mut ec = list(0xffff, 0xffff);      // -1
        ec.ac[0] = 0x0010; ec.ac[1] = 0x0010;
        ec.ac[2] = 0x20;

        assert_eq!(linked_list_search(&mut ec), LKLS_EXIT_FOUND);
        assert_eq!(ec.ac[2], 0x30, "checked word read from link - 1");
    }
}
