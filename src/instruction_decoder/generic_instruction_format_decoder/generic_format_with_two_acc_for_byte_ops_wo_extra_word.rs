// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::ACCUMULATOR_NAMES;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;

pub(crate) const INS: [&str; 7] = ["LDB","STB","RBR","WBR","IBA","STB","LDB"];
pub(crate) const B_ACC_NAMES: [&str; 4] = ["B0","B1","B2","B3"];

pub(super) fn decode(mnemonic : &str, instruction_word: u16, ec: Option<&mut ExecutionContext>) -> String {

    let bc = get_bits(instruction_word,1,2);
    let ac = get_bits(instruction_word,3,4);
    let asm_line = format!("{} {} {}",mnemonic, ACCUMULATOR_NAMES[ac as usize], B_ACC_NAMES[bc as usize]);

    if let Some(ec) = ec {

        match mnemonic {
            "RBR" => ec.ac[ac as usize] = ec.br[(bc - 2) as usize],
            "WBR" => ec.br[(bc - 2)  as usize] = ec.ac[ac as usize],

            "IBA" => {
                let (ac_added_result, ac_added_carry) = ec.ac[ac as usize].carrying_add(1,false);
                ec.ac[ac as usize] = ac_added_result;

                if ac_added_carry {
                    ec.br[(ac - 2) as usize] ^= 0x8000;
                }
            }

            "STB" => {
                let (word_adr,second_byte) = form_abn_byte_adr(bc, ec);
                ec.store_byte_to_mem(word_adr, second_byte, ec.ac[ac as usize] as u8);
            }

            "LDB" => {
                let (word_adr,second_byte) = form_abn_byte_adr(bc, ec);
                let read_byte = ec.load_byte_from_mem(word_adr, second_byte);
                ec.ac[ac as usize] = read_byte as u16;
            }

            _ => todo!("Unimplemented execution model for {}", mnemonic)
        }

        ec.ip += 1; // All of them are one-word instructions
    }

    asm_line
}

pub fn form_abn_byte_adr(bc: u16, ec: &mut ExecutionContext) -> (u16,bool) {
    let mut byte_adr = ec.ac[bc as usize] as u32;
    byte_adr += (ec.br[bc as usize - 2] as u32) << 1;

    let second_byte = byte_adr & 1 == 1;
    let word_adr : u16 = (byte_adr >> 1) as u16;

    (word_adr,second_byte)
}