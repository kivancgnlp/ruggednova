// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::io_format_data_fields;
use crate::instruction_decoder::alc_format_data_fields::Accumulators;
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::{io_device_emulator, ExecutionContext};
use crate::instruction_decoder::io_format_data_fields::Transfer;

const SPECIAL_IO_DEVICE_CODES: [u8;3] = [0,1,0x3f];



pub(super) fn decode(instruction_word: u16, execution_context: Option<&mut ExecutionContext>) -> String {
    assert_eq!(get_bits(instruction_word,0,2), 3);

    let acc = Accumulators::from(get_bits(instruction_word,3,4) as u8);
    let transfer = io_format_data_fields::Transfer::from(get_bits(instruction_word, 5, 7) as u8);
    let control = get_bits(instruction_word,8,9) as u8;
    let io_device = get_bits(instruction_word,10,15) as u8;


    let opcode_is_skip = transfer == Transfer::SKP;
    let opcode_is_no_io = transfer == Transfer::NIO;


    let mut ambiguous_instruction = false;

    if opcode_is_no_io && acc != Accumulators::AC0{
        eprintln!("NIO instruction with a accumulator specified, possible misinterpretation of higher instruction version");
        ambiguous_instruction = true;
    }
    if opcode_is_skip && acc != Accumulators::AC0{
        eprintln!("SKP instruction with a accumulator specified, possible misinterpretation of higher instruction version");
        ambiguous_instruction = true;
    }

    if SPECIAL_IO_DEVICE_CODES.contains(&io_device){
        eprintln!("IO Format with special IO device address, possible misinterpretation of higher instruction version");
        ambiguous_instruction = true;

    }


    let control_str = match control {
        0 => "",
        1 => "S",
        2 => "C",
        3 => "P",
        _ => unreachable!("control unrecognized")
    };

    let skip_suffix_str = match control {
        0 => "BN",
        1 => "BZ",
        2 => "DN",
        3 => "DZ",
        _ => unreachable!("control unrecognized")
    };



    let mut ass_str;
    if opcode_is_skip {
        ass_str = format!("{}{} {}", transfer, skip_suffix_str, io_device);
    }else if opcode_is_no_io {
        ass_str = format!("{}{}, {:#x}", transfer, control_str, io_device);
    }else{
        //Standard IO
         ass_str = format!("{}{} {}, {:#o}", transfer, control_str, acc, io_device);
    }


    if let Some(known_peripheral) = lookup_peripheral(io_device){
        ass_str.push_str(format!(" ({})", known_peripheral).as_str());
    }

    if ambiguous_instruction {
        ass_str.insert(0,'?');
    }

    if let Some(ec) = execution_context {

        if transfer.requires_data_transfer(){
            io_device_emulator::emulate_io_device(io_device, transfer.get_io_device_target_register(), transfer.is_read(), acc,  ec, lookup_peripheral(io_device));
        }
        if opcode_is_skip && test_io_skip_condition(io_device, control, ec) {
            ec.ip += 1;
        }
        ec.ip += 1;
    }

    ass_str
}



/// The CPU's own I/O device code. Table 3-2 lists 77 as "CPU — Central Processing Unit".
const CPU_DEVICE_CODE: u8 = 0o77;

/// The four I/O skips, 3-106..3-107. Bits 8-9 select the test:
///
/// | bits | mnemonic | ordinary device      | device 77 (CPU SKIP, 3-110) |
/// |------|----------|----------------------|-----------------------------|
/// | 00   | SKPBN    | test for Busy set    | test for ION = 1            |
/// | 01   | SKPBZ    | test for Busy clear  | test for ION = 0            |
/// | 10   | SKPDN    | test for Done set    | test for Power Fail = 1     |
/// | 11   | SKPDZ    | test for Done clear  | test for Power Fail = 0     |
///
/// Previously only `SKPDN` was handled, and it skipped *unconditionally*.
fn test_io_skip_condition(io_device: u8, control: u8, ec: &ExecutionContext) -> bool {

    let (busy, done) = read_device_flags(io_device, ec);

    match control {
        0 => busy,
        1 => !busy,
        2 => done,
        3 => !done,
        _ => unreachable!("control unrecognized"),
    }
}

/// Returns (Busy, Done) for a device.
///
/// For device 77 these are the processor's own flags: Busy is the Interrupt On flag and Done is
/// the Power Fail flag (3-110). INS64 group H, H16A at 004176 uses `SKPBZ CPU` to check whether
/// the Stack Overflow Trap cleared ION.
///
/// For every other device code there is no emulated interface yet (finding A3), so both flags read
/// as clear — the state of a device that is idle or simply not attached. That is a placeholder,
/// not a model: a test that waits for Done on a real peripheral will spin here. It is still
/// strictly better than the previous unconditional skip, which fabricated a Done that no device
/// had set.
fn read_device_flags(io_device: u8, ec: &ExecutionContext) -> (bool, bool) {

    if io_device == CPU_DEVICE_CODE {
        return (ec.ion, ec.power_fail);
    }

    // TODO (A3): per-device Busy/Done once io_device_emulator models interfaces.
    (false, false)
}

fn lookup_peripheral(device_code:u8) -> Option<&'static str> {

    match device_code {
        // Table 3-2. Naming these keeps the trace readable now that the four CPU skips are decoded
        // as ordinary SKPxx instructions rather than as prose pseudo-mnemonics.
        0o00 => Some("PWRFL"),
        0o01 => Some("MDV"),
        0o10 => Some("TTI"),
        0o11 => Some("TTO"),
        0o77 => Some("CPU"),
        _ => None,
    }

}

#[cfg(test)]
mod tests {

    #[test]
    fn test_01(){
        let instruction_word = 0o61434_u16;
        let decoded_instruction = super::decode(instruction_word,None);
        assert_eq!(decoded_instruction,"DIB AC0, 0o34")
    }

    #[test]
    fn test_02(){
        let instruction_word = 0o63402;
        let decoded_instruction = super::decode(instruction_word,None);
        assert_eq!(decoded_instruction,"SKPBN 2")


    }

    #[test]
    fn test_03(){
        let instruction_word = 0o60100;
        let decoded_instruction = super::decode(instruction_word,None);
        assert_eq!(decoded_instruction,"?NIOS, 0x0 (PWRFL)")   // device 0 is Power Fail, Table 3-2


    }

    #[test]
    fn ambiguous_instruction_test_01(){
        let instruction_word = 0x68A0; // LASH instruction
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn ambiguous_instruction_test_02(){
        let instruction_word = 0x7040; // FLDS instruction
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn ambiguous_instruction_test_03(){
        let instruction_word = 0o60177; // INTEN instruction
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn ambiguous_instruction_test_04(){
        let instruction_word = 0o70102; // FLDS instruction (floating point)
        let decoded_instruction = super::decode(instruction_word,None);
        assert!(decoded_instruction.starts_with("?"));
    }

    #[test]
    fn test_instruction_scratch_pad(){
        let instruction_word = 0x68A0;
        let decoded_instruction = super::decode(instruction_word,None);
        println!("{:?}", decoded_instruction);


    }
}
#[cfg(test)]
mod cpu_skips {
    use crate::virtual_machine::ExecutionContext;

    fn run(instruction_word: u16, ion: bool, power_fail: bool) -> u16 {
        let mut ec = ExecutionContext::new();
        ec.load_initial_memory(vec![0u16; 0x40]);
        ec.ion = ion;
        ec.power_fail = power_fail;
        ec.ip = 0x100;
        super::decode(instruction_word, Some(&mut ec));
        ec.ip - 0x100          // 1 = no skip, 2 = skipped
    }

    /// CPU SKIP, 3-110. On device 77, Busy is the Interrupt On flag:
    /// BN tests ION = 1, BZ tests ION = 0.
    ///
    /// INS64 group H, H16A at 004176 uses `SKPBZ CPU` to ask whether the Stack Overflow Trap
    /// cleared ION.
    #[test]
    fn skpbn_and_skpbz_on_the_cpu_test_ion() {
        const SKPBN_CPU: u16 = 0o063477;
        const SKPBZ_CPU: u16 = 0o063577;

        assert_eq!(run(SKPBN_CPU, true, false), 2, "SKPBN skips when ION is set");
        assert_eq!(run(SKPBN_CPU, false, false), 1);

        assert_eq!(run(SKPBZ_CPU, false, false), 2, "SKPBZ skips when ION is clear");
        assert_eq!(run(SKPBZ_CPU, true, false), 1);
    }

    /// On device 77, Done is the Power Fail flag: DN tests it set, DZ tests it clear.
    /// `SKPDN` used to skip unconditionally for every device.
    #[test]
    fn skpdn_and_skpdz_on_the_cpu_test_power_fail() {
        const SKPDN_CPU: u16 = 0o063677;
        const SKPDZ_CPU: u16 = 0o063777;

        assert_eq!(run(SKPDN_CPU, false, true), 2);
        assert_eq!(run(SKPDN_CPU, false, false), 1, "no power failure, so no skip");

        assert_eq!(run(SKPDZ_CPU, false, false), 2);
        assert_eq!(run(SKPDZ_CPU, false, true), 1);
    }

    /// An unemulated device reads as Busy = 0, Done = 0 (finding A3). The point of the test is
    /// that SKPDN no longer fabricates a Done flag no device ever set.
    #[test]
    fn an_unemulated_device_reads_as_idle() {
        const SKPDN_DEV_12: u16 = 0o063612;
        const SKPBZ_DEV_12: u16 = 0o063512;

        assert_eq!(run(SKPDN_DEV_12, false, false), 1, "Done is clear, so no skip");
        assert_eq!(run(SKPBZ_DEV_12, false, false), 2, "Busy is clear, so skip");
    }
}
