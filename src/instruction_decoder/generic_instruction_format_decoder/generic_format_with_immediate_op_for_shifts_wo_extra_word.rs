// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::alc_format_instruction_executor::do_left_shift;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;

pub(crate) const B4_INS: [&str; 6] = ["LDSHD","LLSH","RLSH","LLSHD","RLSHD","LROT"];
pub(crate) const B3_INS: [&str; 4] = ["LASH","RASH","LASHD","RASHD"];

pub(crate) const B6_INS: [&str; 2] = ["ADNI","ADPI"];

pub(super) fn decode_4bit(mnemonic : &str, instruction_word: u16,execution_context: Option<&mut ExecutionContext>) -> String {

    let n = get_bits(instruction_word,12,15);
    let asm_str = format!("{} {}",mnemonic, n);

    if let Some(ec) = execution_context {

        match mnemonic {
            "LROT" => {
                //LEFT ROTATE
                let shift_amount = get_bits(instruction_word,12,15) as u8;

                for _ in 0..shift_amount{
                    let (shifted,carry) = do_left_shift(ec.ac[0]);

                    ec.ac[0] = shifted;
                    if carry{
                        ec.ac[0] |= 1;
                    }

                }


            }

            "RLSHD" => {
                //RIGHT LOGICAL SHIFT, DOUBLE
                let shift_amount = get_bits(instruction_word,12,15) as u8;
                let ac01 = ec.get_ac01_compound();
                let ac01_shifted = ac01 >> shift_amount;

                ec.set_ac01_compound(ac01_shifted);

            }

            "LLSHD" => {
                //LEFT LOGICAL SHIFT, DOUBLE
                let shift_amount = get_bits(instruction_word,12,15) as u8;
                let ac01 = ec.get_ac01_compound();
                let ac01_shifted = ac01 << shift_amount;

                ec.set_ac01_compound(ac01_shifted);

            }

            "RLSH" => {
                //RIGHT LOGICAL SHIFT
                let shift_amount = get_bits(instruction_word,12,15) as u8;
                ec.ac[0] >>= shift_amount;

            },

            "LLSH" => {
                //LEFT LOGICAL SHIFT
                let shift_amount = get_bits(instruction_word,12,15) as u8;
                ec.ac[0] <<= shift_amount;

            }

            "LDSHD" => {
                // LEFT DUAL-MODE SHIFT, manual p. 3-65:
                //   "Copy (AC0) to (AC1). Logically left shift (AC1) by n (0 <= n <= 17 octal)
                //    positions. Circularly shift (AC0) by n (0 <= n <= 17 octal) positions. Carry
                //    and Overflow are not affected."
                //
                // Both results derive from the ORIGINAL (AC0): AC1 keeps the bits that survive a
                // plain left shift, AC0 keeps all sixteen bits rotated. The difference between them
                // is what fell off the left, which is what makes this a useful field-extraction
                // primitive. n is the 4-bit field, so 0..=15 — no shift-overflow to guard against.
                let shift_amount = get_bits(instruction_word,12,15) as u32;
                let original_ac0 = ec.ac[0];

                ec.ac[1] = original_ac0 << shift_amount;
                ec.ac[0] = original_ac0.rotate_left(shift_amount);
            }

            _ => {
                todo!("Unimplemented {}", &asm_str);
            }
        }


        ec.ip += 1;  // All of them are one-word instructions that don't change IP
    }
    asm_str
}

pub(super) fn decode_3bit(mnemonic : &str, instruction_word: u16, execution_context: Option<&mut ExecutionContext>) -> String {

    let n = get_bits(instruction_word,13,15);
    let asm_str = format!("{} {}",mnemonic, n);

    if let Some(ec) = execution_context {

        match mnemonic {

            "RASH" => {
                let ac0 = ec.ac[0] as i16;
                let ac0 = ac0 >> n;
                ec.ac[0] = ac0 as u16;
            }
            "LASH" => {
                // LEFT ARITHMETIC SHIFT, manual p. 3-64:
                //   "Shift (AC0) left n (0 <= n <= 7) positions. Zeros are shifted in from the
                //    right. If bit 0 of (AC0) changes, set Carry and Overflow, and restore (AC0) to
                //    its original value. If bit 0 does not change, clear Carry."
                //
                // The sign test has to be applied after EVERY single-position shift, not once to
                // the final result. INS02 proves it at 05422: LASH 7 on 0x2000 shifts the set bit
                // straight out through the sign position and lands on 0x0000, whose sign matches
                // the original — a final-value comparison sees no change and reports no overflow,
                // but the tape's TCO at 05434 expects Overflow set and calls its error routine.
                const SIGN: u16 = 0x8000;
                let original = ec.ac[0];
                let sign_before = original & SIGN;

                let mut value = original;
                let mut sign_changed = false;

                for _ in 0..n {
                    value <<= 1;
                    if value & SIGN != sign_before {
                        sign_changed = true;
                    }
                }

                if sign_changed {
                    ec.carry_flag = true;
                    ec.overflow_flag = true;
                    ec.ac[0] = original;          // restore
                } else {
                    ec.ac[0] = value;
                    ec.carry_flag = false;        // Overflow left alone on this path
                }
            }

            "LASHD" => {
                // LEFT ARITHMETIC SHIFT, DOUBLE, manual p. 3-64:
                //   "Shift (AC01) left n (0 <= n <= 7) positions. High-order bits of (AC1) are
                //    shifted into low-order bits of (AC0), and 0's are shifted into vacated
                //    positions of (AC1). If bit 0 of (AC0) changes, set Carry and Overflow, and
                //    restore (AC0) and (AC1) to their original values. If bit 0 does not change,
                //    clear Carry."
                //
                // Bit 0 of AC0 is the sign of the 32-bit quantity. The test is applied after every
                // single-position shift rather than only to the final result: the hardware is
                // microcoded as an n-step loop, and a sign that flips and flips back still
                // overflowed. Final-value comparison would miss exactly that case.
                let original = ec.get_ac01_compound();
                const SIGN: u32 = 0x8000_0000;
                let sign_before = original & SIGN;

                let mut value = original;
                let mut sign_changed = false;

                for _ in 0..n {
                    value <<= 1;
                    if value & SIGN != sign_before {
                        sign_changed = true;
                    }
                }

                if sign_changed {
                    ec.carry_flag = true;
                    ec.overflow_flag = true;
                    ec.set_ac01_compound(original);   // restore both accumulators
                } else {
                    ec.set_ac01_compound(value);
                    ec.carry_flag = false;            // Overflow left alone on this path
                }
            }

            "RASHD" => {
                // RIGHT ARITHMETIC SHIFT, DOUBLE, manual p. 3-65:
                //   "Shift (AC01) right n (0 <= n <= 7) positions. Low-order bits of (AC0) are
                //    shifted into high-order bits of (AC1). If (AC0) was originally positive, shift
                //    in 0's from the left. If (AC0) was negative, shift in 1's. Carry and Overflow
                //    are not affected."
                //
                // That is a plain arithmetic right shift of the 32-bit pair, which Rust's `>>` on
                // a signed type already performs as a sign-propagating shift.
                let ac01 = ec.get_ac01_compound() as i32;
                ec.set_ac01_compound((ac01 >> n) as u32);
            }

            _ => {
                todo!("Unimplemented {}", &asm_str);
            }
        }

        ec.ip += 1;
    }

    asm_str
}

pub(super) fn decode_6bit(mnemonic : &str, instruction_word: u16, execution_context: Option<&mut ExecutionContext>) -> String {

    let n = get_bits(instruction_word,10,15);
    let asm_str = format!("{} {}",mnemonic, n);

    if let Some(ec) = execution_context {

        match mnemonic {
            "ADPI" =>{
                ec.ac[2] = ec.ac[2].wrapping_add(n);
            }

            "ADNI" =>{
                let intermediate = n as i16 - 64_i16;
                ec.ac[2] = ec.ac[2].wrapping_add(intermediate as u16);
            }

            _ => {
                todo!("Unimplemented {}", &asm_str);
            }
        }

        ec.ip += 1;
    }

    asm_str
}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use super::*;
    #[test]
    fn rlshd_shift_test(){
        let mut ec = ExecutionContext::new();

        ec.ac[0] = 0x200;
        ec.ac[1] = 0x0001;
        decode_4bit("RLSHD", 0x687a, Some(&mut ec));

        assert_eq!(ec.ac[1], 0x8000_u16);
        //println!("{}",ec)
    }

    #[test]
    fn rash_shift_test(){
        let mut ec = ExecutionContext::new();

        ec.ac[0] = 0x8000;

        let mut instruction_word = 0x68A8_u16; // RASH instruction

        set_bits(&mut instruction_word,13,15,3); // shift  by 3

        decode_3bit("RASH",instruction_word , Some(&mut ec));

        assert_eq!(ec.ac[0], 0xf000_u16);
        //println!("{}",ec)
    }

    #[test]
    fn lash_shift_test(){
        let mut ec = ExecutionContext::new();

        ec.ac[0] = 0x2000;

        let mut instruction_word = 0x68A0_u16; // LASH instruction

        set_bits(&mut instruction_word,13,15,1); // shift  by 1
        decode_3bit("LASH",instruction_word , Some(&mut ec));
        assert_eq!(ec.ac[0], 0x4000_u16);
        assert_eq!(ec.carry_flag, false);
        assert_eq!(ec.overflow_flag, false);

        set_bits(&mut instruction_word,13,15,1); // shift  by 1
        decode_3bit("LASH",instruction_word , Some(&mut ec));
        assert_eq!(ec.ac[0], 0x4000_u16);
        assert_eq!(ec.carry_flag, true);
        assert_eq!(ec.overflow_flag, true);


        //println!("{}",ec)
    }

    #[test]
    fn lrot_shift_test(){
        let mut ec = ExecutionContext::new();

        ec.ac[0] = 0x4000;

        let mut instruction_word = 0x6880_u16; // RASH instruction

        set_bits(&mut instruction_word,13,15,2); // shift  by 2

        decode_4bit("LROT",instruction_word , Some(&mut ec));

        assert_eq!(ec.ac[0], 1);
        //println!("{}",ec)
    }
}
#[cfg(test)]
mod shift_conformance {
    use super::*;
    use crate::virtual_machine::ExecutionContext;

    fn ctx() -> ExecutionContext {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0; 32]);
        ec
    }
    // Canonical octal from Appendix E, with n in the low bits.
    const fn lash(n: u16)  -> u16 { 0o064240 | n }
    const fn lashd(n: u16) -> u16 { 0o064140 | n }
    const fn rashd(n: u16) -> u16 { 0o064150 | n }
    const fn ldshd(n: u16) -> u16 { 0o064100 | n }

    /// Manual p. 3-64. The sign test applies after every single-position shift, not to the final
    /// result. INS02 at 05422: LASH 7 on 0x2000 walks the set bit out through the sign position and
    /// lands on 0x0000, whose sign matches the original — a final-value comparison reports no
    /// overflow, but the tape's TCO at 05434 expects Overflow set.
    #[test]
    fn lash_detects_a_sign_change_that_does_not_survive_to_the_result() {
        let mut ec = ctx();
        ec.ac[0] = 0x2000;
        ec.overflow_flag = false;
        decode_3bit("LASH", lash(7), Some(&mut ec));
        assert!(ec.carry_flag, "Carry set on overflow");
        assert!(ec.overflow_flag, "Overflow set on overflow");
        assert_eq!(ec.ac[0], 0x2000, "AC0 restored to its original value");
    }

    /// The non-overflowing case clears Carry and keeps the result.
    #[test]
    fn lash_without_a_sign_change_keeps_the_result() {
        let mut ec = ctx();
        ec.ac[0] = 0x2000;
        ec.carry_flag = true;
        decode_3bit("LASH", lash(1), Some(&mut ec));
        assert_eq!(ec.ac[0], 0x4000);
        assert!(!ec.carry_flag, "Carry cleared when bit 0 does not change");

        // A negative value whose sign holds throughout.
        let mut ec = ctx();
        ec.ac[0] = 0xFE00;
        decode_3bit("LASH", lash(5), Some(&mut ec));
        assert_eq!(ec.ac[0], 0xC000);
        assert!(!ec.carry_flag);
    }

    /// Manual p. 3-64: shift AC01 left, high bits of AC1 into low bits of AC0, zeros into AC1.
    /// On a change of bit 0 of AC0, set Carry and Overflow and restore BOTH accumulators.
    #[test]
    fn lashd_shifts_the_pair_and_restores_both_on_overflow() {
        let mut ec = ctx();
        // 0x01234567 << 4 stays positive at every step, so no overflow.
        ec.ac[0] = 0x0123; ec.ac[1] = 0x4567;
        decode_3bit("LASHD", lashd(4), Some(&mut ec));
        assert_eq!(ec.ac[0], 0x1234, "high nibble of AC1 shifted into AC0");
        assert_eq!(ec.ac[1], 0x5670, "zeros shifted into AC1");
        assert!(!ec.carry_flag);

        // 0x12348765 << 4 exceeds the 32-bit signed range: the sign flips mid-shift even though
        // the final value is positive again. Per-step detection catches it; a final comparison
        // would not.
        let mut ec = ctx();
        ec.ac[0] = 0x1234; ec.ac[1] = 0x8765;
        decode_3bit("LASHD", lashd(4), Some(&mut ec));
        assert!(ec.carry_flag && ec.overflow_flag, "transient sign change is still an overflow");
        assert_eq!((ec.ac[0], ec.ac[1]), (0x1234, 0x8765), "both accumulators restored");

        let mut ec = ctx();
        ec.ac[0] = 0x4000; ec.ac[1] = 0x0000;
        ec.overflow_flag = false;
        decode_3bit("LASHD", lashd(2), Some(&mut ec));
        assert!(ec.carry_flag && ec.overflow_flag);
        assert_eq!((ec.ac[0], ec.ac[1]), (0x4000, 0x0000), "both accumulators restored");
    }

    /// Manual p. 3-65: arithmetic right shift of the pair, sign of AC0 propagated in from the left.
    /// Carry and Overflow are not affected.
    #[test]
    fn rashd_propagates_the_sign_and_leaves_the_flags_alone() {
        let mut ec = ctx();
        ec.ac[0] = 0xFF00; ec.ac[1] = 0x0F00;
        ec.carry_flag = true; ec.overflow_flag = true;
        decode_3bit("RASHD", rashd(4), Some(&mut ec));
        assert_eq!(ec.ac[0], 0xFFF0, "1s shifted in because AC0 was negative");
        assert_eq!(ec.ac[1], 0x00F0, "low bits of AC0 shifted into AC1");
        assert!(ec.carry_flag && ec.overflow_flag, "flags untouched");

        let mut ec = ctx();
        ec.ac[0] = 0x0F00; ec.ac[1] = 0x00F0;
        decode_3bit("RASHD", rashd(4), Some(&mut ec));
        assert_eq!(ec.ac[0], 0x00F0, "0s shifted in because AC0 was positive");
        assert_eq!(ec.ac[1], 0x000F);
    }

    /// Manual p. 3-65: copy AC0 to AC1, logically left shift AC1 by n, circularly shift AC0 by n.
    /// Both results derive from the original AC0; the difference between them is the bits that
    /// wrapped around, which is what makes this a field-extraction primitive.
    #[test]
    fn ldshd_leaves_a_rotated_ac0_and_a_shifted_ac1() {
        let mut ec = ctx();
        ec.ac[0] = 0xABCD; ec.ac[1] = 0x0000;
        ec.carry_flag = true; ec.overflow_flag = true;
        decode_4bit("LDSHD", ldshd(4), Some(&mut ec));
        assert_eq!(ec.ac[0], 0xBCDA, "AC0 rotated left 4");
        assert_eq!(ec.ac[1], 0xBCD0, "AC1 is the original AC0 shifted left 4");
        assert!(ec.carry_flag && ec.overflow_flag, "Carry and Overflow are not affected");

        // n = 0 is the identity on both.
        let mut ec = ctx();
        ec.ac[0] = 0xABCD; ec.ac[1] = 0x1111;
        decode_4bit("LDSHD", ldshd(0), Some(&mut ec));
        assert_eq!((ec.ac[0], ec.ac[1]), (0xABCD, 0xABCD));
    }
}
