// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::assembler::common_assembler_utils;
use crate::instruction_decoder::alc_format_data_fields::{Accumulators, AlcCarryField, AlcShiftField, AlcSkipField};
use crate::instruction_decoder::bit_utils::set_bits;

pub(super) fn build(asm_line:&str, instruction_word_base:u16) -> Option<u16>{

    let mut split_whitespace_iter = asm_line.split_whitespace();
    let no_load = asm_line.contains('#');
    split_whitespace_iter.next()?; // skip mnemonic part

    let mut second_part = split_whitespace_iter.next()?;
    if second_part == "#" { // if second part only no_load char skip to ACS,ACD field
        second_part = split_whitespace_iter.next()?
    }

    let (carry_field,shift_field);
    let no_cs_part;
    if second_part.contains("AC"){
        no_cs_part = true;
        carry_field = AlcCarryField::N;
        shift_field = AlcShiftField::NoShift

    }else{
        no_cs_part = false;
        (carry_field,shift_field)= parse_carry_and_shift_fields(second_part)?;
    }

    let third_part = if no_cs_part {second_part} else {split_whitespace_iter.next()?};
    
    let (acs,acd) = common_assembler_utils::parse_accumulators(third_part)?;

    let end_part = split_whitespace_iter.next();

    let skip = match end_part {
        Some(end_part) => {
            parse_skip_part(end_part)
        }
        None => AlcSkipField::NoSkip

    };


    return Some(encode_alc_format_ins(instruction_word_base, acs, acd, shift_field, carry_field, no_load, skip));


}


fn encode_alc_format_ins(base_word: u16, acs: Accumulators, acd: Accumulators, shift: AlcShiftField, carry: AlcCarryField, no_load: bool, skip: AlcSkipField) -> u16{

    let mut instruction_word = base_word;
    set_bits(&mut instruction_word, 1, 2, acs as u16);
    set_bits(&mut instruction_word, 3, 4, acd as u16);
    set_bits(&mut instruction_word, 8, 9, shift as u16);
    set_bits(&mut instruction_word, 10, 11, carry as u16);
    set_bits(&mut instruction_word, 12, 12, if no_load {1} else {0});
    set_bits(&mut instruction_word, 13, 15, skip as u16);

    debug_assert_ne!( (no_load && skip == AlcSkipField::NoSkip) , true);

    instruction_word
}

fn parse_skip_part(skip_str: &str) -> AlcSkipField {

    if skip_str.contains("SKP"){
        return AlcSkipField::SKP;
    }

    if skip_str.contains("SZC"){
        return AlcSkipField::SZC;
    }

    if skip_str.contains("SNC"){
        return AlcSkipField::SNC;
    }

    if skip_str.contains("SZR"){
        return AlcSkipField::SZR;
    }

    if skip_str.contains("SNR"){
        return AlcSkipField::SNR;
    }

    if skip_str.contains("SEZ"){
        return AlcSkipField::SEZ
    }

    if skip_str.contains("SBN"){
        return AlcSkipField::SBN;
    }


    AlcSkipField::NoSkip
}



fn parse_carry_and_shift_fields(second_part: &str) -> Option<(AlcCarryField, AlcShiftField)> {
    let zero_carry = second_part.contains('Z');
    let one_carry = second_part.contains('O');
    let complement_carry = second_part.contains('C');
    if count_bools(zero_carry, one_carry, complement_carry) > 1 {
        return None; // These flags are mutually exclusive
    }

    let carry_field;
    if zero_carry {
        carry_field = AlcCarryField::Z;
    } else if one_carry {
        carry_field = AlcCarryField::O
    } else if complement_carry {
        carry_field = AlcCarryField::C
    } else {
        carry_field = AlcCarryField::N
    }


    let shift_left = second_part.contains('L');
    let shift_right = second_part.contains('R');
    let shift_swap = second_part.contains('S');

    if count_bools(shift_left, shift_right, shift_swap) > 1 {
        return None; // These flags are mutually exclusive
    }

    let shift_field;

    if shift_left {
        shift_field = AlcShiftField::L;
    } else if shift_right {
        shift_field = AlcShiftField::R;
    } else if shift_swap {
        shift_field = AlcShiftField::S;
    } else {
        shift_field = AlcShiftField::NoShift
    }
    Some((carry_field,shift_field))
}

fn count_bools(b1:bool, b2:bool,b3:bool) -> u8 {
    let mut count = 0_u8;

    if b1 {
        count += 1;
    }

    if b2 {
        count += 1;
    }
    if b3 {
        count += 1;
    }

    count
}
