// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::alc_format_data_fields::{AlcCarryField, AlcFunctionField, AlcShiftField, AlcSkipField};
use crate::virtual_machine::ExecutionContext;



pub(super) fn execute_alu_op(acs_value:u16, acd_value:u16, carry_initial_setting:AlcCarryField, shift_setting:AlcShiftField, no_load:bool, dest_acc_id:u8, skip : AlcSkipField, ec: &mut ExecutionContext, function : AlcFunctionField){

    let carry_in = match carry_initial_setting {
        AlcCarryField::N => ec.carry_flag,
        AlcCarryField::Z => false,
        AlcCarryField::O => true ,
        AlcCarryField::C => !ec.carry_flag,
    };


    let (function_result, function_carry_out) =
    match function {
        AlcFunctionField::ADD => acs_value.carrying_add(acd_value, false),
        AlcFunctionField::ADC => {
            let one_complement = acs_value ^ 0xffff_u16;
            one_complement.carrying_add(acd_value, false)
        },
        AlcFunctionField::INC => acs_value.carrying_add(1, false),
        AlcFunctionField::SUB => {
            let res = acd_value.borrowing_sub(acs_value, false);
            (res.0, !res.1)
        },
        AlcFunctionField::NEG => {
            let minus = 0_u16.borrowing_sub(acs_value, false); // subtract from zero
            (minus.0, acs_value == 0) // carry only generated when source operand is zero
        },
        AlcFunctionField::COM => (acs_value ^ 0xffff_u16,false), // Complement
        AlcFunctionField::MOV => (acs_value,false),
        AlcFunctionField::AND => (acs_value & acd_value,false),    

    };

    let function_carry_sum = carry_in ^ function_carry_out;


    let (shifter_result,shifter_carry_out) = match shift_setting {
        AlcShiftField::NoShift => (function_result, function_carry_sum),
        AlcShiftField::L => {
            let (mut shifted_val, shifted_result_carry) = do_left_shift(function_result);

            if function_carry_sum {
                  shifted_val |= 1;
             }

            (shifted_val, shifted_result_carry)
        },
        AlcShiftField::R => {
            let (mut shifted_val, shifted_result_carry) = do_right_shift(function_result);


            if function_carry_sum{
                shifted_val |= 0x8000;
            }

           (shifted_val,shifted_result_carry)
        },

        AlcShiftField::S => (function_result.swap_bytes(), function_carry_sum),
    };

    match function {
        AlcFunctionField::ADD => {
            ec.overflow_flag = check_overflow(acs_value, acd_value, function_result, true);
        }

        AlcFunctionField::INC => {
            ec.overflow_flag = check_overflow(acs_value, 1, function_result, true);
        }

        AlcFunctionField::SUB | AlcFunctionField::ADC => {
            ec.overflow_flag = check_overflow(acs_value, acd_value, function_result, false);
        }

        AlcFunctionField::NEG => {
            ec.overflow_flag = acs_value == 0x8000;
        }



        _ => {
            // Overflow flag not affected in logical operations other than add or sub
        }
    }

    ec.zero_flag = shifter_result == 0;
    if !no_load{
        ec.carry_flag = shifter_carry_out;
        ec.ac[dest_acc_id as usize] = shifter_result;
    }

    let do_skip = match skip {
        AlcSkipField::NoSkip => false, // No skip
        AlcSkipField::SKP => true,  // Always skip
        AlcSkipField::SZC => shifter_carry_out == false, // skip if carry zero
        AlcSkipField::SNC => shifter_carry_out == true, // skip if carry one
        AlcSkipField::SZR => shifter_result == 0, //skip if result is zero
        AlcSkipField::SNR => shifter_result != 0, //skip if result is non zero
        AlcSkipField::SEZ => shifter_carry_out == false || shifter_result == 0 , // TODO : Son ikisi test edilmedi
        AlcSkipField::SBN => shifter_carry_out == true && shifter_result != 0 ,

    };

    ec.ip += 1; // normal ilerleme
    if do_skip{
        ec.ip += 1; // skip varsa bir tane daha
    }

}

pub(crate) fn check_overflow(acs_input_val:u16, acd_input_val:u16, result:u16, addition:bool) -> bool{
    let acs_operand_signed = acs_input_val as i16;
    let acd_operand_signed = acd_input_val as i16;
    let result_signed = result as i16;

    let acs_operand_negative = acs_operand_signed < 0;
    let acd_operand_negative = acd_operand_signed < 0;
    let result_is_negative = result_signed < 0;
    let operands_have_diff_signs = acs_operand_negative ^ acd_operand_negative;
    let operands_have_same_signs = acs_operand_negative == acd_operand_negative;

    if addition{

        return operands_have_same_signs &&  acs_operand_negative != result_is_negative;


    }else{ // Subtraction

        return operands_have_diff_signs &&  acd_operand_negative != result_is_negative;
    }

}

pub(crate) fn do_left_shift(mut value: u16) -> (u16, bool) {

    let mut carry = false;

    if value & 0x8000 == 0x8000 {
        carry = true;
        value &= 0x7fff;
    }

    value <<= 1;

    (value,carry)
}

fn do_right_shift(mut value: u16) -> (u16, bool) {

    let mut carry = false;

    if value & 1 == 1 {
        carry = true;
        value &= 0xfffe;
    }

    value >>= 1;

    (value,carry)
}
