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
                let ac0_initial = ec.ac[0] as i16;

                let ac_positive = ac0_initial >= 0;

                ec.ac[0] <<= n;

                let ac0_after_shift = ec.ac[0] as i16;

                ec.carry_flag = false;

                if ac_positive && ac0_after_shift < 0{ // if sign changes
                    ec.carry_flag = true;
                    ec.overflow_flag = true;
                    ec.ac[0] = ac0_initial as u16;
                }

                if !ac_positive && ac0_after_shift >= 0{// if sign changes
                    ec.carry_flag = true;
                    ec.overflow_flag = true;
                    ec.ac[0] = ac0_initial as u16;
                }

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