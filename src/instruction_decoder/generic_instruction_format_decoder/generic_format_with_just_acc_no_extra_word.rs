// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::alc_format_data_fields::Accumulators;
use crate::instruction_decoder::bit_utils::{get_bits, set_bits};
use crate::virtual_machine::ExecutionContext;

pub(crate) const INS: [&str; 27] = ["RSP","WSP","RFP","WFP","RSL","WSL","POP","IOR","XOR","PSH","DEC","UDVI","SMPY","UMPY","UMPA","WMSR","RMSR","READS","MSKO","RMVR","TRAP","UJMP","BTZ","BTO","SZB","SZBO","COB"];

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
                let quot = ec.ac[1] / ec.ac[target_acc];
                let rem = ec.ac[1] % ec.ac[target_acc];

                ec.ac[0] = rem;
                ec.ac[1] = quot;

                // TODO :  Overflow occurs when the contents of ac are zero or when the divisor is in ACO
                // and (ACO) is equal to one. If overflow occurs set Carry and leave all accumulators unchanged. If
                // there is no overflow clear Carry and leave Overflow unchanged.

            }

            "TRAP" => {
                let trap_no = ec.ac[target_acc];
                println!("Calling trap {}",trap_no);
                ec.call_trap(trap_no as u8);
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
                // Signed multiply
                let ac1 = ec.ac[1] as i32;
                let signed_mult = ac1 * ec.ac[target_acc] as i32;
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