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

    // Overflow is STICKY: an ALC sets it when the operation overflows and never clears it.
    // Only TCO (and IORST) clear it.
    //
    // Two diagnostics pin this down, and only this rule satisfies both:
    //
    //   INS64 group G, G44 at 003256 (listing page 0045):
    //       SUBZR 1,1        ; AC1 = 100000
    //       ADDZ# 1,1,SKP    ; "AN ALC# INSTR CAN ALSO SET OVF"
    //       NOP
    //       TCO              ; must NOT skip -> Overflow is set
    //       JMP .+2
    //       ?EHLT            ; "BUT DIDN'T"
    //   so the no-load (#) bit does NOT suppress the Overflow update. It suppresses only
    //   Carry and (acd), exactly as the instruction pages say.
    //
    //   INS02 at 04632: SDVD (sets Carry and Overflow), then four SUB# ac,ac,SZR accumulator
    //   port checks, then TCO at 04643 which must NOT skip -> Overflow survived all four.
    //   SUB# 0,0 does not overflow, so a non-overflowing ALC must leave Overflow alone rather
    //   than clear it.
    //
    // The manual's wording agrees: instructions are documented as "set Carry and Overflow"
    // (SDVD 3-35, LASH) or "Overflow is unchanged" (UDVD, UDVI) — never "clear Overflow".
    // Clearing is what TCO is for.
    let overflowed = match function {
        AlcFunctionField::ADD => check_overflow(acs_value, acd_value, function_result, true),

        AlcFunctionField::INC => check_overflow(acs_value, 1, function_result, true),

        AlcFunctionField::SUB | AlcFunctionField::ADC => {
            check_overflow(acs_value, acd_value, function_result, false)
        }

        AlcFunctionField::NEG => acs_value == 0x8000,

        // Overflow is not affected by the logical operations.
        _ => false,
    };

    if overflowed {
        ec.overflow_flag = true;
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

#[cfg(test)]
mod no_load_overflow {
    use crate::instruction_decoder::alc_format_instruction_decoder;
    use crate::virtual_machine::ExecutionContext;

    /// INS02 at 04632 runs SDVD (which sets Carry and Overflow), then four `SUB# ac,ac,SZR`
    /// accumulator port checks, then TCO at 04643 which must NOT skip — so Overflow has to
    /// survive all four. `SUB# 0,0` does not overflow, and a non-overflowing ALC must leave
    /// Overflow alone rather than clear it.
    #[test]
    fn a_non_overflowing_alc_does_not_clear_overflow() {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0; 32]);
        ec.overflow_flag = true;
        ec.carry_flag = true;

        // SUB# AC0,AC0 SZR — 0x850c, the standard Nova compare idiom.
        alc_format_instruction_decoder::decode(0x850c, Some(&mut ec));

        assert!(ec.overflow_flag, "Overflow is sticky; nothing overflowed here");
        assert!(ec.carry_flag, "# must leave Carry unchanged");
    }

    /// The same holds without the no-load bit: Overflow is cleared by TCO, never by an ALC.
    #[test]
    fn the_loading_form_also_leaves_overflow_alone() {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0; 32]);
        ec.overflow_flag = true;
        ec.carry_flag = false;
        ec.ac[0] = 1;

        // SUB AC0,AC0 SZR — 0x8504, same instruction with # clear.
        alc_format_instruction_decoder::decode(0x8504, Some(&mut ec));

        assert!(ec.overflow_flag, "1 - 1 does not overflow, so Overflow is untouched");
        assert!(ec.carry_flag, "acd >= acs unsigned complements carry");
        assert_eq!(ec.ac[0], 0);
    }

    /// INS64 group G, G44 at 003256:
    ///
    /// ```text
    ///   003256 126620  SUBZR 1,1       ; AC1 = 0x8000
    ///   003257 127031  ADDZ# 1,1,SKP   ; "AN ALC# INSTR CAN ALSO SET OVF"
    ///   003260 000401  NOP
    ///   003261 063401  TCO             ; must NOT skip
    ///   003262 000402  JMP .+2
    ///                  ?EHLT           ; "BUT DIDN'T"
    /// ```
    ///
    /// 0x8000 + 0x8000 is a signed overflow. The no-load bit suppresses Carry and (acd) only —
    /// Overflow is still set.
    #[test]
    fn no_load_does_not_suppress_setting_overflow() {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0; 32]);
        ec.overflow_flag = false;
        ec.carry_flag = false;
        ec.ac[1] = 0x8000;

        // ADDZ# 1,1,SKP — octal 127031.
        alc_format_instruction_decoder::decode(0o127031, Some(&mut ec));

        assert!(ec.overflow_flag, "ADDZ# on 0x8000 + 0x8000 must set Overflow");
        assert_eq!(ec.ac[1], 0x8000, "# must not write acd");
        assert!(!ec.carry_flag, "# must not write Carry");
    }

    /// G46 at 003264 contrasts it: the corresponding MOVZL must NOT set Overflow, because
    /// Overflow belongs to the adder, not the shifter.
    #[test]
    fn movzl_does_not_set_overflow() {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0; 32]);
        ec.overflow_flag = false;
        ec.ac[1] = 0x8000;

        // MOVZL 1,1 — octal 125120.
        alc_format_instruction_decoder::decode(0o125120, Some(&mut ec));

        assert!(!ec.overflow_flag, "a shift is not an arithmetic overflow");
    }
}
