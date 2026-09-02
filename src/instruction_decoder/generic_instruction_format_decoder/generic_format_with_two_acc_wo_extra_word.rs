// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::alc_format_data_fields::Accumulators;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;

pub(crate) const INS: [&str; 3] = ["SSGE","SSGT","XCH"];

pub(super) fn decode(mnemonic : &str, instruction_word: u16, ec: Option<&mut ExecutionContext>) -> String {

    let acs = Accumulators::from(get_bits(instruction_word, 1, 2) as u8);
    let acd = Accumulators::from(get_bits(instruction_word, 3, 4) as u8);
    let asm_line = format!("{} {} {}", mnemonic, acs, acd);

    if let Some(ec) = ec {

        let acs_value = ec.ac[acs as usize] as i16;
        let acd_value = ec.ac[acd as usize] as i16;

        match mnemonic {
            "SSGE" => {
                //SIGNED SKIP ON GREATER THAN OR EQUAL
                if acs_value >= acd_value {
                    ec.ip += 2;
                }else{
                    ec.ip += 1;
                }

            },
            "SSGT" => {
                // SIGNED SKIP ON GREATER THAN
                if acs_value > acd_value {
                    ec.ip += 2;
                }else{
                    ec.ip += 1;
                }

            },

            "XCH" => {
                // Exchange acs and acd
                ec.ac[acs as usize] = acd_value as u16;
                ec.ac[acd as usize] = acs_value as u16;
                ec.ip += 1;
            }

            _ => todo!("Unimplemented execution model for {}", mnemonic)
        }


    }

    asm_line
}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::set_bits;
    use super::*;
    #[test]
    fn ssge_functional_test() {

        // Compare the signed integers (acs) with (acd). If (acs) >= (acd), increment (PC) by 2
        let mut ins_word = 0_u16;

        set_bits(&mut ins_word, 1, 2, 0); // ACS = AC0
        set_bits(&mut ins_word, 3, 4, 1); // ACD = AC1

        let mut ec = ExecutionContext::new();

        ec.ac[0] = 1; // ACS
        ec.ac[1] = 2; // ACD

        decode("SSGE", ins_word, Some(&mut ec));

        assert_eq!(ec.ip, 1);

        ec.ip = 0;
        ec.ac[0] = 2; // ACS
        ec.ac[1] = 2; // ACD

        decode("SSGE", ins_word, Some(&mut ec));
        assert_eq!(ec.ip, 2);

        ec.ip = 0; // Signed compare test
        ec.ac[0] = -1_i16 as u16; // ACS
        ec.ac[1] = -2_i16 as u16; // ACD
        decode("SSGE", ins_word, Some(&mut ec));
        assert_eq!(ec.ip, 2);


        println!("{}", ec.ip);


        //assert_eq!(str, "JMP @ +23 (17)");
    }
}