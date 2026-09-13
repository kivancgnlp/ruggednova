// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::alc_format_data_fields::Accumulators;
use crate::instruction_decoder::bit_utils::{get_bits, set_bits};
use crate::virtual_machine::ExecutionContext;

pub(crate) const INS: [&str; 31] = ["RSP","WSP","RFP","WFP","RSL","WSL","POP","IOR","XOR","PSH","DEC","UDVI", "SDVD","UDVD","SMPY","UMPY","UMPA","WMSR","RMSR","READS","MSKO","RMVR","TRAP","UJMP","BTZ","BTO","SZB","SZBO","COB","LDF","STF"];

pub(super) fn decode(mnemonic : &str, instruction_word: u16, ec: Option<&mut ExecutionContext>) -> String {

    let ac = Accumulators::from(get_bits(instruction_word,3,4)as u8);
    let abn = get_bits(instruction_word,1,2);
    let asm_str = format!("{} {}",mnemonic, ac);

    if let Some(ec) = ec {

        let target_acc = ac as usize;
        let mut auto_increment_ip = true;

        match mnemonic {
            "RSP" => ec.ac[target_acc] = ec.sp,
            "WSP" => ec.sp = ec.ac[target_acc],
            "RFP" => ec.ac[target_acc] = ec.fp,
            "WFP" => ec.fp = ec.ac[target_acc],
            "RSL" => ec.ac[target_acc] = ec.sl,
            "WSL" => ec.sl = ec.ac[target_acc],

            "IOR" => ec.ac[target_acc] |= ec.ac[0], // IOR Burada hata yok, operandlardan biri hep ilk accumulator
            "XOR" => ec.ac[target_acc] ^= ec.ac[0], // XOR Burada hata yok, operandlardan biri hep ilk accumulator
            "DEC" => ec.ac[target_acc] = ec.ac[target_acc].wrapping_sub(1),
            "PSH" => {
                ec.push_a_single_word_to_the_stack(ec.ac[target_acc]);
            },
            "POP" => {
                ec.ac[target_acc] = ec.pop_a_single_word_from_the_stack();

            },
            "WMSR" => {
                ec.mapping_unit.msr.load_msr_via_wmsr_instruction(ec.ac[target_acc]);
            },

            "RMSR" => {
                ec.ac[target_acc] = ec.mapping_unit.msr.get_msr_word();
            },

            "READS" => {
                //If the control panel is disconnected from the processor the specified ac will be loaded with 177777
                //const READS_RETURN_VALUE: u16 = 0x4001_u16;
                const READS_RETURN_VALUE: u16 = 0xffff_u16;
                println!("READS executed at IP : {:#x} returning {:#x}", ec.ip, READS_RETURN_VALUE);
                ec.ac[target_acc] = READS_RETURN_VALUE;
            }

            "RMVR" => {
                // Reads Map Violation Register to the specified accumulator
                println!("Reads Map Violation Register at IP : {:#x}",ec.ip);
                ec.ac[target_acc] = ec.mapping_unit.mvr.get_mvr_word(ec.mapping_unit.msr.user);
            }

            "MSKO" => {
                let irq_priority_mask = ec.ac[target_acc];
                println!("Updating interrupt priority mask to {:#x}", irq_priority_mask);
                ec.interrupt_priority_mask = irq_priority_mask;
            }

            "UDVI" => {
                // UNSIGNED INTEGER DIVIDE, manual p. 3-36.
                //   "Divide the unsigned integer in AC1 by (ac). Set (AC0) = remainder, set (AC1)
                //    = quotient. Check for overflow. Overflow occurs when the contents of ac are
                //    zero or when the divisor is in AC0 and (AC0) is equal to one. If overflow
                //    occurs set Carry and leave all accumulators unchanged. If there is no
                //    overflow clear Carry and leave Overflow unchanged."
                //
                // Note the dividend is the 16-bit AC1, not the double word AC01 that SDVD and UDVD
                // take. No quotient bound is needed: a 16-bit dividend over a non-zero divisor
                // always fits in 16 bits.
                let dividend = ec.ac[1];
                let divisor  = ec.ac[target_acc];

                // The second clause looks like a microcode artifact — dividing by one cannot
                // overflow arithmetically — and this was left unimplemented as open question X6
                // with a note not to guess at it. INS64 group I settles it at 005172:
                //
                //     005171  MOVZ  2,2
                //     005172  UDVI  0        ; the divisor register IS AC0, and (AC0) == 1
                //     005173  MOV#  2,2,SNC  ; must skip, so Carry has to be set
                //     005174  ?EHLT
                //
                // It makes sense once you notice AC0 is the remainder destination: when AC0 is
                // also the divisor, the microcode has nowhere to keep the divisor while it writes
                // the remainder. The hardware declines rather than producing a wrong answer.
                //
                // It is specifically (AC0) == 1, not any divisor of 1: `UDVI 2` with AC2 == 1 is a
                // perfectly ordinary divide.
                let divisor_register_is_ac0 = target_acc == 0;
                let overflow = divisor == 0 || (divisor_register_is_ac0 && divisor == 1);

                if overflow {
                    ec.carry_flag = true;            // all accumulators unchanged, Overflow too
                } else {
                    ec.ac[0] = dividend % divisor;
                    ec.ac[1] = dividend / divisor;
                    ec.carry_flag = false;           // Overflow unchanged
                }
            }

            "SDVD" => {
                // SIGNED DIVIDE, manual p. 3-35.
                //   "Divide the two's complement signed double word integer (AC01) by the signed
                //    integer (ac). (AC0) = remainder and (AC1) = quotient. The sign of the quotient
                //    is determined by the rules of algebra. The sign of the remainder is always the
                //    same as the sign of the dividend. If the divisor was 0 or the quotient >= 2^15,
                //    set Carry and Overflow and restore (AC01). Otherwise, clear Carry. Overflow is
                //    unchanged."
                //
                // Read every operand before writing anything: target_acc may be 0 or 1, and AC0/AC1
                // are the destinations. "Restore (AC01)" is satisfied by not writing on the
                // overflow path.
                let dividend = ec.get_ac01_compound() as i32;   // 32-bit, AC0 high : AC1 low
                let divisor  = ec.ac[target_acc] as i16 as i32; // 16-bit, sign extended

                // The quotient bound is the real overflow test: a 32-bit dividend over a 16-bit
                // divisor can easily produce a quotient that will not fit back into AC1.
                //
                // The manual writes the limit as "the quotient >= 2^15", which reads as a MAGNITUDE
                // bound rather than the signed i16 range. INS02 confirms it: the case at 04674
                // divides 0x00020000 by -4, giving exactly -32768. That value is representable in
                // i16, so a range check accepts it — but the tape's TCO at 04677 expects Overflow
                // to be set and calls its error routine when it is not. So -32768 overflows too.
                let overflow = if divisor == 0 {
                    true
                } else {
                    let quot = dividend / divisor;
                    quot >= 32768 || quot <= -32768
                };

                if overflow {
                    // SDVD is the only divide that writes Overflow. UDVD and UDVI both say
                    // "Overflow is unchanged" — do not copy this line into them.
                    ec.carry_flag = true;
                    ec.overflow_flag = true;
                } else {
                    // Rust's / and % truncate toward zero, which gives the quotient the sign the
                    // rules of algebra require and the remainder the sign of the dividend.
                    let quot = dividend / divisor;
                    let rem  = dividend % divisor;

                    ec.ac[0] = rem  as u16;
                    ec.ac[1] = quot as u16;

                    ec.carry_flag = false;   // Overflow left untouched on this path
                }
            }

            "UDVD" => {
                // UNSIGNED DIVIDE, manual p. 3-35.
                //   "If (AC0) >= (ac), unsigned, set Carry and proceed to the next instruction;
                //    otherwise, divide the unsigned double-word integer (AC01) by the unsigned
                //    single-word integer (ac). Set (AC0) = remainder, set (AC1) = quotient, and
                //    clear Carry. Overflow is unchanged."
                //
                // The precondition doubles as the quotient-overflow guard: AC0 < divisor is exactly
                // the condition under which AC01 / divisor fits in 16 bits. It also rules out
                // divisor == 0, since AC0 >= 0 always holds.
                let dividend = ec.get_ac01_compound();          // u32, AC0 high : AC1 low
                let divisor  = ec.ac[target_acc] as u32;

                if ec.ac[0] as u32 >= divisor {
                    ec.carry_flag = true;   // accumulators and Overflow unchanged
                } else {
                    ec.ac[0] = (dividend % divisor) as u16;
                    ec.ac[1] = (dividend / divisor) as u16;
                    ec.carry_flag = false;  // Overflow unchanged
                }
            }

            "TRAP" => {
                // Manual p. 3-101 and the note under Table 2-4: "The four least significant bits of
                // the specified accumulator are used as INDEX in the system trap sequence." The trap
                // table is 20(octal) = 16 entries, so anything wider indexes past the end of it.
                let trap_no = (ec.ac[target_acc] & 0x000F) as u8;
                println!("Calling trap {}",trap_no);
                ec.call_trap(trap_no);

                // call_trap loads the PC with the trap handler's entry address and pushes ip+1 as
                // the return address. Falling through to the auto-increment below would start the
                // handler one instruction in.
                auto_increment_ip = false;
            }

            "UJMP" => {
                debug_assert!(ec.mapping_unit.msr.is_executive(),"This privileged instruction and should be executed as executive");

                ec.mapping_unit.msr.user_mode = true;


                if ec.mapping_unit.msr.user < 2{
                    eprintln!("UJMP Bug 1 ?");
                }


                ec.ip = ec.ac[target_acc];
                auto_increment_ip = false;
            }

            "BTZ" => {
                //SET BIT TO ZERO
                change_bit(ec, abn as usize, false);
            }

            "LDF" => {
                // LOAD BIT FIELD, manual p. 3-16..3-17.
                //   AC1 = length of bit field, ABn = bit field address.
                //   "The LDF instruction loads AC0 with the bit field pointed to by (abn). The
                //    length of the bit field is specified by the low order four bits of AC1 (if 0,
                //    a field of length 16 will be loaded). The result is right justified in AC0."
                //
                // The field is read most-significant bit first, starting AT the bit address, and
                // may run across word boundaries — the manual's own worked example unpacks a
                // 14/8/10 split of a 32-bit double word, where the 8-bit field straddles the two
                // words.
                //
                // Note what does NOT happen: the bit address is not advanced. That example steps
                // it by hand with `ADD 1,2` between the loads.
                let length = bit_field_length(ec.ac[1]);
                let base = form_bit_field_address(ec, target_acc);

                let mut field = 0_u16;
                for i in 0..length {
                    field = (field << 1) | read_bit_at(ec, base.wrapping_add(i)) as u16;
                }

                ec.ac[0] = field;   // right justified; Carry and Overflow unaffected
            }

            "STF" => {
                // STORE BIT FIELD, manual p. 3-18.
                //   AC0 = bit field to be stored, AC1 = length, ABn = bit field address.
                //   "Store the right justified bit field in AC0 at the bit field address given by
                //    (abn). The length of the bit field is given by the low order four bits of AC1
                //    (if 0, a field of length 16 will be stored)."
                //
                // The exact inverse of LDF: bit 0 of the stored field is the field's most
                // significant bit, which lives at the LOW end of the right-justified value in AC0.
                let length = bit_field_length(ec.ac[1]);
                let base = form_bit_field_address(ec, target_acc);
                let field = ec.ac[0];

                for i in 0..length {
                    let bit = (field >> (length - 1 - i)) & 1 != 0;
                    write_bit_at(ec, base.wrapping_add(i), bit);
                }
            }

            "BTO" => {
                //SET BIT TO ONE
                change_bit(ec, abn as usize,true);
            }

            "SZB" => {
                //SKIP IF BIT ZERO
                let bit = get_bit(ec, abn as usize);

                if bit == false{
                    ec.ip += 2;
                    auto_increment_ip = false;
                }
            }

            "SZBO" => {
                //SKIP IF BIT ZERO, SET TO ONE
                let bit_before_change = get_bit(ec, abn as usize);

                if bit_before_change == false{
                    change_bit(ec, abn as usize, true);
                    ec.ip += 2;
                    auto_increment_ip = false;
                }
            }

            "COB" => {
                //COUNT ONE BITS
                let one_count = u16::count_ones(ec.ac[0]) as u16;
                ec.ac[1] += one_count;

            }

            "UMPY" => {
                let ac1 = ec.ac[1] as u32;
                let unsigned_mult = ac1 * ec.ac[target_acc] as u32;
                ec.set_ac01_compound(unsigned_mult);
            }

            "SMPY" => {
                // Signed multiply: reinterpret the u16 bits as i16, sign-extend to i32, then multiply
                let ac1 = ec.ac[1] as i16 as i32;
                let acn = ec.ac[target_acc] as i16 as i32;
                let signed_mult: i32 = ac1 * acn;
                ec.set_ac01_compound(signed_mult as u32);
            }


            "UMPA" => {
                let ac1 = ec.ac[1] as u32;
                let unsigned_mult = ac1 * ec.ac[target_acc] as u32;
                let u_mult_add = unsigned_mult + ec.ac[0] as u32;
                ec.set_ac01_compound(u_mult_add);
            }

            _ => todo!("Uimplemented execution model for {}", mnemonic)
        }

        if auto_increment_ip {
            ec.ip += 1;
        }

    }

    asm_str
}

/// Manual p. 3-17: "A bit field address is the sum of (bn) four bits left shifted and (acn)",
/// giving a 20-bit address — 65,536 words of 16 bits each. Word `n` of memory owns bit addresses
/// `16n .. 16n+15`, and within a word bit 0 is the most significant, the same numbering the whole
/// manual uses.
fn form_bit_field_address(ec: &ExecutionContext, target_acc: usize) -> u32 {
    const BIT_FIELD_ADDRESS_MASK: u32 = 0x0f_ffff;   // 20 bits

    let bit_adr = (ec.ac[target_acc] as u32) + ((ec.br[target_acc - 2] as u32) << 4);
    bit_adr & BIT_FIELD_ADDRESS_MASK
}

/// "The length of the bit field is specified by the low order four bits of AC1 (if 0, a field of
/// length 16 will be loaded)." (3-17, and identically for STF on 3-18.)
fn bit_field_length(ac1: u16) -> u32 {
    match ac1 & 0x0f {
        0 => 16,
        n => n as u32,
    }
}

fn read_bit_at(ec: &mut ExecutionContext, bit_adr: u32) -> bool {
    let word = ec.mapping_unit.read_word_from_memory((bit_adr >> 4) as u16, true);
    let index = (bit_adr & 0xf) as u8;
    get_bits(word, index, index) != 0
}

fn write_bit_at(ec: &mut ExecutionContext, bit_adr: u32, value: bool) {
    let word_adr = (bit_adr >> 4) as u16;
    let mut word = ec.mapping_unit.read_word_from_memory(word_adr, true);
    let index = (bit_adr & 0xf) as u8;
    set_bits(&mut word, index, index, value as u16);
    ec.mapping_unit.write_word_to_memory(word_adr, word, true);
}

fn change_bit(ec: &mut ExecutionContext, target_acc: usize, val:bool) {
    let mut bit_adr = ec.ac[target_acc] as u32;
    bit_adr += (ec.br[target_acc - 2] as u32) << 4;

    let mut data_word = ec.mapping_unit.read_word_from_memory((bit_adr >> 4) as u16, true);

    let bit_index = (bit_adr & 0xf) as u8;
    set_bits(&mut data_word, bit_index, bit_index, val as u16);

    ec.mapping_unit.write_word_to_memory((bit_adr >> 4) as u16, data_word, true);
}

fn get_bit(ec: &mut ExecutionContext, target_acc: usize) -> bool {
    let mut bit_adr = ec.ac[target_acc] as u32;
    bit_adr += (ec.br[target_acc - 2] as u32) << 4;

    let data_word = ec.mapping_unit.read_word_from_memory((bit_adr >> 4) as u16, true);

    let bit_index = (bit_adr & 0xf) as u8;
    get_bits(data_word, bit_index, bit_index) != 0
}

#[cfg(test)]
mod tests {

    use super::*;

    // ---- Conformance tests for the fixes: SDVD / UDVD / UDVI / TRAP ----
    //
    // Instruction words are built from the canonical octal in Appendix E with the ac field
    // (bits 3-4, i.e. value << 11) filled in.
    const fn with_ac(base: u16, ac: u16) -> u16 { base | (ac << 11) }

    fn ctx() -> ExecutionContext {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0; 64]);
        ec
    }

    /// Manual p. 3-35: the dividend is the 32-bit double word AC01, not AC1 alone.
    #[test]
    fn sdvd_divides_the_full_ac01_double_word() {
        let mut ec = ctx();
        ec.ac[0] = 0x0001; ec.ac[1] = 0x0000;   // AC01 = 65536
        ec.ac[2] = 256;
        decode("SDVD", with_ac(0o061377, 2), Some(&mut ec));
        assert_eq!(ec.ac[1], 256, "quotient");
        assert_eq!(ec.ac[0], 0, "remainder");
        assert!(!ec.carry_flag);
    }

    /// "The sign of the quotient is determined by the rules of algebra. The sign of the remainder
    /// is always the same as the sign of the dividend."
    #[test]
    fn sdvd_signs_follow_the_manual() {
        // -7 / 2 => quotient -3, remainder -1 (truncation toward zero)
        let mut ec = ctx();
        ec.set_ac01_compound((-7_i32) as u32);
        ec.ac[2] = (2_i16) as u16;
        decode("SDVD", with_ac(0o061377, 2), Some(&mut ec));
        assert_eq!(ec.ac[1] as i16, -3, "quotient");
        assert_eq!(ec.ac[0] as i16, -1, "remainder takes the dividend's sign");

        // 7 / -2 => quotient -3, remainder +1
        let mut ec = ctx();
        ec.set_ac01_compound(7);
        ec.ac[2] = (-2_i16) as u16;
        decode("SDVD", with_ac(0o061377, 2), Some(&mut ec));
        assert_eq!(ec.ac[1] as i16, -3, "quotient");
        assert_eq!(ec.ac[0] as i16, 1, "remainder takes the dividend's sign");
    }

    /// "If the divisor was 0 or the quotient >= 2^15, set Carry and Overflow and restore (AC01)."
    #[test]
    fn sdvd_overflow_sets_both_flags_and_restores_ac01() {
        // Quotient too large for AC1.
        let mut ec = ctx();
        ec.set_ac01_compound(0x00FF_FFFF);
        ec.ac[2] = 2;
        ec.overflow_flag = false;
        decode("SDVD", with_ac(0o061377, 2), Some(&mut ec));
        assert!(ec.carry_flag, "Carry set on quotient overflow");
        assert!(ec.overflow_flag, "SDVD is the one divide that also sets Overflow");
        assert_eq!(ec.get_ac01_compound(), 0x00FF_FFFF, "AC01 restored");

        // Divisor of zero.
        let mut ec = ctx();
        ec.set_ac01_compound(100);
        ec.ac[2] = 0;
        ec.overflow_flag = false;
        decode("SDVD", with_ac(0o061377, 2), Some(&mut ec));
        assert!(ec.carry_flag);
        assert!(ec.overflow_flag);
        assert_eq!(ec.get_ac01_compound(), 100, "AC01 restored");
    }

    /// The divisor may be AC0 or AC1, which are also the destinations, so it has to be latched
    /// before either is written.
    #[test]
    fn sdvd_reads_the_divisor_before_writing_the_destinations() {
        let mut ec = ctx();
        ec.ac[0] = 0x0000; ec.ac[1] = 100;   // AC01 = 100, and AC1 is also the divisor
        decode("SDVD", with_ac(0o061377, 1), Some(&mut ec));
        assert_eq!(ec.ac[1] as i16, 1, "100 / 100");
        assert_eq!(ec.ac[0] as i16, 0);
    }


    /// INS02 at 04674: 0x00020000 / -4 == exactly -32768. That value fits in an i16, so a signed
    /// RANGE check accepts it — but the tape's TCO at 04677 expects Overflow to be set. So the
    /// manual's "quotient >= 2^15" is a magnitude bound, not a range bound.
    #[test]
    fn sdvd_treats_a_quotient_of_exactly_minus_2_to_the_15_as_overflow() {
        let mut ec = ctx();
        ec.ac[0] = 0x0002; ec.ac[1] = 0x0000;   // AC01 = 131072
        ec.ac[3] = (-4_i16) as u16;
        ec.overflow_flag = false;
        decode("SDVD", with_ac(0o061377, 3), Some(&mut ec));
        assert!(ec.carry_flag, "Carry set");
        assert!(ec.overflow_flag, "Overflow set — |quotient| == 2^15");
        assert_eq!(ec.get_ac01_compound(), 0x0002_0000, "AC01 restored");

        // One less in magnitude must still succeed.
        let mut ec = ctx();
        ec.set_ac01_compound(32767 * 4);
        ec.ac[3] = 4;
        decode("SDVD", with_ac(0o061377, 3), Some(&mut ec));
        assert!(!ec.carry_flag);
        assert_eq!(ec.ac[1] as i16, 32767);
    }

    /// Manual p. 3-35: "If (AC0) >= (ac), unsigned, set Carry and proceed to the next instruction;
    /// otherwise, divide the unsigned double-word integer (AC01) ..."
    #[test]
    fn udvd_divides_ac01_and_guards_on_ac0() {
        // AC0 < divisor: the divide runs.
        let mut ec = ctx();
        ec.ac[0] = 0x0001; ec.ac[1] = 0x0000;   // AC01 = 65536
        ec.ac[2] = 3;
        ec.overflow_flag = true;
        decode("UDVD", with_ac(0o063101, 2), Some(&mut ec));
        assert_eq!(ec.ac[1], 21845, "quotient");
        assert_eq!(ec.ac[0], 1, "remainder");
        assert!(!ec.carry_flag);
        assert!(ec.overflow_flag, "Overflow is unchanged");

        // AC0 >= divisor: quotient would not fit, so skip the divide and set Carry.
        let mut ec = ctx();
        ec.ac[0] = 5; ec.ac[1] = 0;
        ec.ac[2] = 4;
        decode("UDVD", with_ac(0o063101, 2), Some(&mut ec));
        assert!(ec.carry_flag);
        assert_eq!(ec.ac[0], 5, "accumulators unchanged");
        assert_eq!(ec.ac[1], 0);

        // A zero divisor is covered by the same guard: AC0 >= 0 always holds.
        let mut ec = ctx();
        ec.ac[0] = 0; ec.ac[1] = 9; ec.ac[2] = 0;
        decode("UDVD", with_ac(0o063101, 2), Some(&mut ec));
        assert!(ec.carry_flag);
        assert_eq!(ec.ac[1], 9);
    }

    /// Manual p. 3-36: a zero divisor sets Carry and leaves all accumulators unchanged.
    #[test]
    fn udvi_zero_divisor_is_not_a_panic() {
        let mut ec = ctx();
        ec.ac[0] = 0xAAAA; ec.ac[1] = 1234; ec.ac[2] = 0;
        decode("UDVI", with_ac(0o063201, 2), Some(&mut ec));
        assert!(ec.carry_flag);
        assert_eq!(ec.ac[0], 0xAAAA, "accumulators unchanged");
        assert_eq!(ec.ac[1], 1234);

        let mut ec = ctx();
        ec.ac[1] = 1234; ec.ac[2] = 100;
        decode("UDVI", with_ac(0o063201, 2), Some(&mut ec));
        assert_eq!(ec.ac[1], 12);
        assert_eq!(ec.ac[0], 34);
        assert!(!ec.carry_flag);
    }

    /// Manual p. 3-36, UDVI's second overflow clause: "Overflow occurs when the contents of ac are
    /// zero **or when the divisor is in AC0 and (AC0) is equal to one**."
    ///
    /// This was open question X6 — the wording looks like a microcode artifact, since dividing by
    /// one cannot overflow arithmetically. INS64 group I settles it at 005172:
    ///
    /// ```text
    ///   005171  MOVZ  2,2
    ///   005172  UDVI  0        ; the divisor register IS AC0, and (AC0) == 1
    ///   005173  MOV#  2,2,SNC  ; must skip, so Carry has to be set
    ///   005174  ?EHLT
    /// ```
    #[test]
    fn udvi_overflows_when_the_divisor_register_is_ac0_holding_one() {
        let mut ec = ctx();
        ec.ac[0] = 1;
        ec.ac[1] = 1234;
        ec.overflow_flag = true;

        decode("UDVI", with_ac(0o063201, 0), Some(&mut ec));

        assert!(ec.carry_flag, "divisor in AC0 equal to one must set Carry");
        assert_eq!(ec.ac[0], 1, "all accumulators unchanged");
        assert_eq!(ec.ac[1], 1234);
        assert!(ec.overflow_flag, "Overflow is unchanged either way");
    }

    /// It is specifically (AC0) == 1 with AC0 as the divisor register. A divisor of one in any
    /// OTHER accumulator is an ordinary divide.
    #[test]
    fn udvi_dividing_by_one_in_another_accumulator_is_ordinary() {
        let mut ec = ctx();
        ec.ac[1] = 1234; ec.ac[2] = 1;

        decode("UDVI", with_ac(0o063201, 2), Some(&mut ec));

        assert!(!ec.carry_flag, "no overflow when the divisor register is not AC0");
        assert_eq!(ec.ac[1], 1234, "quotient");
        assert_eq!(ec.ac[0], 0, "remainder");
    }

    /// AC0 as the divisor register is fine for any other value — only one is special.
    #[test]
    fn udvi_with_ac0_as_divisor_works_for_other_values() {
        let mut ec = ctx();
        ec.ac[0] = 100; ec.ac[1] = 1234;

        decode("UDVI", with_ac(0o063201, 0), Some(&mut ec));

        assert!(!ec.carry_flag);
        assert_eq!(ec.ac[1], 12);
        assert_eq!(ec.ac[0], 34);
    }

    /// Manual p. 3-101 / Table 2-4: only the four least significant bits index the trap table.
    #[test]
    fn trap_masks_the_index_to_four_bits() {
        let mut ec = ctx();
        ec.mapping_unit.write_word_to_memory(2, 0x30, true);         // TSTAB
        ec.mapping_unit.write_word_to_memory(3, 0x20, true);         // SDTAB
        ec.mapping_unit.write_word_to_memory(0x30 + 1, 0xBEEF, true); // trap 1 handler
        ec.mapping_unit.write_word_to_memory(0x30 + 0x11, 0xDEAD, true); // would-be trap 0x11
        ec.sp = 0x40; ec.sl = 0x10;

        ec.ac[2] = 0x0011;   // low four bits are 1
        decode("TRAP", with_ac(0o120110, 2), Some(&mut ec));
        assert_eq!(ec.ip, 0xBEEF, "0x11 & 0xF == 1, so trap 1");
    }

    #[test]
    fn push_pop_test(){
        let mut ec = ExecutionContext::new();

        ec.load_initial_memory(vec![0;10]);

        ec.ac[1] = 0x1234;

        ec.sp = 5;

        decode("PSH", 0x6a41, Some(&mut ec));

        ec.ac[1] = 0;

        decode("POP", 0x6a81, Some(&mut ec));

        assert_eq!(ec.sp, 5);
        assert_eq!(ec.ac[1], 0x1234);
        println!("{}",ec)
    }
}
#[cfg(test)]
mod bit_fields {
    use super::*;

    const LDF_AB2: u16 = 0x7600;
    const STF_AB2: u16 = 0x7640;

    fn ctx() -> ExecutionContext {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 0x100]);
        ec
    }

    /// The manual's own worked example, p. 3-17: unpack a 32-bit code split into three fields of
    /// 14, 8 and 10 bits, right-justifying each into AC0.
    ///
    /// ```text
    ///   START: LEF 2,PAC     ;LOAD PAC ADDRESS
    ///          WBR 2,2       ;SET BASE REGISTER
    ///          SUB 2,2       ;ZERO AC2
    ///          LEF 1,14.     ;14 TO AC1
    ///          LDF 2         ;FIELD 1 TO AC0
    ///          ADD 1,2       ;ADD 14 TO AB2
    ///          ...
    /// ```
    ///
    /// The 8-bit middle field straddles the word boundary, which is the part worth testing: the
    /// field is read most-significant bit first from the bit address and simply runs on into the
    /// next word.
    #[test]
    fn ldf_unpacks_the_manuals_three_field_example() {
        let mut ec = ctx();
        // 0xAAAB33C3 = 14 bits 0x2AAA | 8 bits 0xCC | 10 bits 0x3C3
        ec.mapping_unit.write_word_to_memory(0x20, 0xAAAB, true);
        ec.mapping_unit.write_word_to_memory(0x21, 0x33C3, true);

        ec.br[0] = 0x20;          // WBR 2,2 — the base register holds the word address
        ec.ac[2] = 0;             // SUB 2,2 — bit offset zero

        ec.ac[1] = 14;
        decode("LDF", LDF_AB2, Some(&mut ec));
        assert_eq!(ec.ac[0], 0x2AAA, "field 1, right justified");

        ec.ac[2] += 14;           // ADD 1,2
        ec.ac[1] = 8;
        decode("LDF", LDF_AB2, Some(&mut ec));
        assert_eq!(ec.ac[0], 0x00CC, "field 2 straddles the word boundary");

        ec.ac[2] += 8;
        ec.ac[1] = 10;
        decode("LDF", LDF_AB2, Some(&mut ec));
        assert_eq!(ec.ac[0], 0x03C3, "field 3");
    }

    /// "if 0, a field of length 16 will be loaded" — only the low four bits of AC1 are the length,
    /// so 0x10 and 0x20 mean 16 too.
    #[test]
    fn a_length_of_zero_means_sixteen() {
        let mut ec = ctx();
        ec.mapping_unit.write_word_to_memory(0x30, 0x1234, true);
        ec.br[0] = 0x30;
        ec.ac[2] = 0;

        ec.ac[1] = 0;
        decode("LDF", LDF_AB2, Some(&mut ec));
        assert_eq!(ec.ac[0], 0x1234);

        ec.ac[1] = 0xfff0;        // high bits are ignored; the low four are still zero
        ec.ac[0] = 0;
        decode("LDF", LDF_AB2, Some(&mut ec));
        assert_eq!(ec.ac[0], 0x1234);
    }

    /// STF is the exact inverse of LDF, and must disturb nothing outside the field.
    #[test]
    fn stf_writes_only_the_field() {
        let mut ec = ctx();
        ec.mapping_unit.write_word_to_memory(0x40, 0xffff, true);
        ec.mapping_unit.write_word_to_memory(0x41, 0xffff, true);

        ec.br[0] = 0x40;
        ec.ac[2] = 4;             // start four bits in
        ec.ac[1] = 8;
        ec.ac[0] = 0x00a5;

        decode("STF", STF_AB2, Some(&mut ec));

        assert_eq!(ec.mapping_unit.read_word_from_memory(0x40, true), 0xfa5f,
                   "bits 4-11 replaced, the rest untouched");
        assert_eq!(ec.mapping_unit.read_word_from_memory(0x41, true), 0xffff,
                   "the next word is not touched");
    }

    /// Round-trip across a word boundary, in both accumulator flavours.
    #[test]
    fn stf_then_ldf_round_trips_across_a_word_boundary() {
        for (ldf, stf, br_index, ac_index) in [(LDF_AB2, STF_AB2, 0usize, 2usize),
                                               (0x7E00u16, 0x7E40u16, 1usize, 3usize)] {
            let mut ec = ctx();
            ec.br[br_index] = 0x50;
            ec.ac[ac_index] = 12;     // 12 bits in, so a 10-bit field crosses into the next word
            ec.ac[1] = 10;
            ec.ac[0] = 0x0355;

            decode("STF", stf, Some(&mut ec));

            ec.ac[0] = 0;
            decode("LDF", ldf, Some(&mut ec));
            assert_eq!(ec.ac[0], 0x0355, "round trip through AB{}", ac_index);
        }
    }

    /// Manual p. 3-17: "A bit field address is the sum of (bn) four bits left shifted and (acn)",
    /// so the accumulator is a bit offset within the base register's word.
    #[test]
    fn the_bit_field_address_is_the_base_register_shifted_by_four_plus_the_accumulator() {
        let mut ec = ctx();
        ec.mapping_unit.write_word_to_memory(0x61, 0xf000, true);

        ec.br[0] = 0x60;
        ec.ac[2] = 16;            // one whole word past the base
        ec.ac[1] = 4;

        decode("LDF", LDF_AB2, Some(&mut ec));

        assert_eq!(ec.ac[0], 0xf, "read from word 0x61, not 0x60");
    }
}
