// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::alc_format_data_fields::{Accumulators, AlcCarryField, AlcFunctionField, AlcShiftField, AlcSkipField};
use crate::instruction_decoder::bit_utils::get_bits;
use crate::virtual_machine::ExecutionContext;
use crate::instruction_decoder::alc_format_instruction_executor;


/// Decodes a given 16-bit instruction word and returns its assembly language representation
/// as a `String`. This function processes an instruction word adhering to the ALC (Arithmetic
/// and Logical Control) type format.
///
/// # Parameters
/// - `instruction_word` (`u16`): The 16-bit instruction word being decoded. It must meet
///   specific requirements (e.g., MSB must be `1`, and specific bits must not conflict).
///
/// # Returns
/// `String`: A formatted assembly-language-style representation of the instruction,
/// describing the operation, source accumulator, destination accumulator, optional
/// shifts, carry operations, and applicable skips.
///
/// # Panics
/// This function will panic in the following scenarios:
/// - If the MSB (Most Significant Bit) of `instruction_word` is not set to `1`.
/// - If the instruction indicates both "NO LOAD" and "NO SKIP" (which is invalid).
/// # Example
/// ```
/// let instruction: u16 = 0x8A5C; // Example 16-bit instruction word
/// let decoded_instruction = decode_alc_type_instruction(instruction);
/// println!("{}", decoded_instruction);
/// assert_eq(3,4);
/// // Output: ALC operation string in assembly format
/// ```
///

pub(crate) fn decode(instruction_word: u16, execution_context: Option<&mut ExecutionContext>) -> String {

    assert_eq!(get_bits(instruction_word,0,0),1); // Sanity checks for ALC (MSB bit should be 1)

    let source_acc = Accumulators::from(get_bits(instruction_word,1,2) as u8);
    let dest_acc = Accumulators::from(get_bits(instruction_word,3,4) as u8);
    let function = AlcFunctionField::from(get_bits(instruction_word,5,7) as u8);
    let shift = AlcShiftField::from(get_bits(instruction_word,8,9) as u8);
    let carry = AlcCarryField::from(get_bits(instruction_word,10,11) as u8);
    let no_load = get_bits(instruction_word, 12, 12) == 1;
    let skip = AlcSkipField::from(get_bits(instruction_word, 13, 15) as u8);

    // An ALC that both suppresses the load and never skips computes a result, stores it nowhere,
    // and cannot branch — a legal no-op that the 1969 Nova left free. Every descendant filled this
    // hole with its own extended instructions, so reaching THIS decoder with the hole set means no
    // extended-instruction table entry matched, i.e. the word is one of Appendix D's unimplemented
    // codes. On real hardware that raises the Unimplemented Instruction Trap rather than executing
    // as an ALC.
    //
    // Instruction identification is most-specific-mask-wins, so a genuine ROLM extended instruction
    // never arrives here; it is decoded by its own table row.
    let unimplemented_instruction = no_load && skip == AlcSkipField::NoSkip;
    if unimplemented_instruction {
        eprintln!("No load and no skip in ALC instruction, possible misinterpretation of higher instruction version");
    }
    let ambiguous_instruction = unimplemented_instruction;


    let mut instruction_asm_str = String::new();
    instruction_asm_str.push_str(function.to_string().as_str());
    instruction_asm_str.push_str(" ");

    let mut mid_part_exist = false;
    if carry != AlcCarryField::N{
        instruction_asm_str.push_str(carry.to_string().as_str());
        mid_part_exist = true;
    }

    if shift != AlcShiftField::NoShift{
        instruction_asm_str.push_str(shift.to_string().as_str());
        mid_part_exist = true;
    }

    if no_load {
        instruction_asm_str.push_str("#");
        mid_part_exist = true;
    }

    if mid_part_exist{
        instruction_asm_str.push_str(" ");
    }



    instruction_asm_str.push_str(source_acc.to_string().as_str());
    instruction_asm_str.push_str(",");
    instruction_asm_str.push_str(dest_acc.to_string().as_str());


    if skip != AlcSkipField::NoSkip{
        instruction_asm_str.push_str(" ");
        instruction_asm_str.push_str(skip.to_string().as_str());
    }

    if ambiguous_instruction {
        instruction_asm_str.insert(0, '?');
    }

    if let Some(ec) = execution_context {

        if unimplemented_instruction {
            // Section 2.29. The instruction must NOT execute — the trap replaces it. `ec.ip` still
            // points at this instruction here, which is exactly the address location 42 wants.
            //
            // INS64 group Q, Q02A at 006057, plants `155130` ("AN UNIMP. INSTR. MOVZL# 2,3") for
            // precisely this and then checks that its handler ran once and that location 42 held
            // 006057 before the handler's ISZ stepped it to 006060.
            ec.unimplemented_instruction_trap(ec.ip);
            return instruction_asm_str;
        }

        let (acs_value, acd_value) = (ec.ac[source_acc as usize], ec.ac[dest_acc.clone()  as usize]);

        alc_format_instruction_executor::execute_alu_op(acs_value, acd_value, carry, shift, no_load, dest_acc.into(), skip, ec, function);



    }

    instruction_asm_str

}

#[cfg(test)]
mod tests {
    use std::io::{stdout, Error};
    use crate::assembler::Assembler;
    use crate::instruction_decoder;
    use crate::instruction_decoder::alc_format_instruction_decoder::decode;
    use crate::virtual_machine::ExecutionContext;
    use crate::instruction_identifier::InstructionIdentifier;

    #[test]
    #[should_panic]
    fn not_valid_instruction() {
        decode(0, None);
    }
    #[test]
    //#[should_panic]
    fn illegal_alc_instruction_with_no_load_and_no_skip() {
        let decoded = decode(0x8008, None);
        assert!(decoded.starts_with("?"));
    }

    #[test]
    fn decode_simple_instruction_01() {
        let decoded = decode(0x8500, None);
        assert_eq!(decoded, "SUB AC0,AC0");
    }

    #[test]
    fn decode_simple_instruction_02() {
        let decoded = decode(0xB09A, None);
        //println!("{}",decoded);
        assert_eq!(decoded, "COM ZR# AC1,AC2 SZC(skip if no carry)");
    }

    #[test]
    fn decode_simple_instruction_03() {
        let decoded = decode(0xc248, None);
        println!("{}",decoded);
        //assert_eq!(decoded, "COM ZR# AC1,AC2 SZC");
    }

    #[test]
    fn functional_test_01() {

        let mut ex = ExecutionContext::new();
        let decoded = decode(0x8d0c, Some(&mut ex)); //SUB # AC0,AC1 SZR(skip if zero)
        println!("{}",decoded);
        //println!("{:?}",ex);

    }

    #[test]
    fn functional_addition_test() -> Result<(), Error> {

        let mut ex = ExecutionContext::new();
        let assembler = Assembler::new()?;

        ex.ac[0] = 3;
        ex.ac[1] = 4;

        let test_asm_line = "ADD AC0,AC1";
        let ins = assembler.assemble_line(test_asm_line).ok_or(Error::other("Unable to assemble line"))?;
        let decoded = decode(ins[0], Some(&mut ex));
        //println!("{}",decoded);

        assert_eq!(ex.ac[1], 7);
        assert_eq!(ex.ip, 1);
        Ok(())

    }

    #[test]
    fn functional_subtraction_test() -> Result<(), Error> {

        let mut ex = ExecutionContext::new();
        let assembler = Assembler::new()?;

        ex.ac[2] = 5;
        ex.ac[3] = 5;

        let test_asm_line = "SUB AC2,AC3 SZR";
        let ins = assembler.assemble_line(test_asm_line).ok_or(Error::other("Unable to assemble line"))?;
        let decoded = decode(ins[0], Some(&mut ex));
        //println!("{}",decoded);
        //println!("{:?}",ex);

        assert_eq!(ex.ac[3], 0);
        assert_eq!(ex.ip, 2);
        Ok(())
    }

    #[test]
    fn functional_increment_with_shift() -> Result<(), Error> {

        let mut ex = ExecutionContext::new();
        let assembler = Assembler::new()?;


        let test_asm_line = "INC L AC0,AC0";
        let ins = assembler.assemble_line(test_asm_line).ok_or(Error::other("Unable to assemble line"))?;
        decode(ins[0], Some(&mut ex));

        //println!("{:?}",ex);

        assert_eq!(ex.ac[0], 2);
        Ok(())
    }

    #[test]
    fn functional_overflowing_and_carry_test() -> Result<(), Error> {

        let mut ex = ExecutionContext::new();
        let assembler = Assembler::new()?;

        ex.ac[0] = 0xe000;
        ex.ac[1] = 0xc000;

        let test_asm_line = "ADD AC0,AC1";
        let ins = assembler.assemble_line(test_asm_line).ok_or(std::io::Error::other("Unable to assemble line"))?;
        decode(ins[0], Some(&mut ex));

        //println!("{:?}",ex);
        assert_eq!(ex.carry_flag, true);
        ex.carry_flag = false;

        ex.ac[0] = 0x7000;
        ex.ac[1] = 0x7000;
        decode(ins[0], Some(&mut ex));
        //println!("{:?}",ex);
        assert_eq!(ex.overflow_flag, true);
        Ok(())
    }

    #[test]
    fn functional_acc_negative_check_test() -> Result<(), Error> {

        let mut ex = ExecutionContext::new();
        let assembler = Assembler::new()?;

        ex.ac[0] = 0x8001;

        let test_asm_line = "MOV ZL AC0,AC0 SNC"; // Skip if no carry (sign bit shifts into a carry flag)
        let ins = assembler.assemble_line(test_asm_line).ok_or(Error::other("Unable to assemble line"))?;
        decode(ins[0], Some(&mut ex));

        assert_eq!(ex.ip, 2);
        Ok(())
    }

    #[test]
    fn multiply_by_10_asm_scenario() -> std::io::Result<()> {
        
        let instruction_identifer = InstructionIdentifier::new()?;
        let assembler = Assembler::new_using_instruction_identifier(&instruction_identifer)?;

        // test code fragment multiplies the input AC0 by 10.
        let test_asm_line = "LDFNW AC0 0x03\n\
        MOV ZL AC0,AC1\n\
        ADD AC1,AC1\n\
        ADD ZL AC1,AC0\n";


        let mut ex = ExecutionContext::new();
        ex.load_initial_memory(assembler.assemble_lines(test_asm_line));

        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ex)?;
        instruction_decoder.decode_instructions(4)?;
        assert_eq!(instruction_decoder.peek_ec().ac[0], 30);

        Ok(())
    }

    #[test]
    fn subtraction_test() -> std::io::Result<()> {


        let instruction_identifer = InstructionIdentifier::new()?;
        let assembler = Assembler::new_using_instruction_identifier(&instruction_identifer)?;

        let test_asm_line = "LDFNW AC0 0x03\n\
        LDFNW AC1 0x1\n\
        NEG AC1,AC1\n\
        ADD AC0,AC1\n\
        SUB AC1,AC1\n\
        COM AC1,AC1\n\
        ADD AC0,AC1\n";


        let mut ex = ExecutionContext::new();
        ex.load_initial_memory(assembler.assemble_lines(test_asm_line));
        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ex)?;
        instruction_decoder.decode_instructions(7)?;
        println!("{}",instruction_decoder.peek_ec());
        assert_eq!(instruction_decoder.peek_ec().ac[1], 2);

        Ok(())
    }
    #[test]
    fn bit_rotate_around_carry_alc_instruction_test() -> std::io::Result<()> {


        let instruction_identifer = InstructionIdentifier::new()?;
        let assembler = Assembler::new_using_instruction_identifier(&instruction_identifer)?;


        let test_asm_line = "\
        SUB ZL AC2,AC2 ; Loads 1 to AC2\n\
        MOV R AC2,AC2 ; Right shift\n\
        MOV R AC2,AC2 ; Right shift\n";

        let mut ex = ExecutionContext::new();
        ex.load_initial_memory(assembler.assemble_lines(test_asm_line));

        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ex)?;
        instruction_decoder.decode_instructions(3)?;
        println!("{}",instruction_decoder.peek_ec());
        assert_eq!(instruction_decoder.peek_ec().ac[2],0x8000);


        Ok(())
    }

    #[test]
    fn literal_generating_alc_instructions_test() -> std::io::Result<()> {


        let instruction_identifer = InstructionIdentifier::new()?;
        let assembler = Assembler::new_using_instruction_identifier(&instruction_identifer)?;

        //
        //
        let test_asm_line = "\
        SUB ZL AC2,AC2 ; Loads 1 to AC2\n\
        MOV AC2,AC1\n\
        ADC AC1,AC1 ; Loads -1 to AC1\n\
        ADC ZL AC3,AC3 ; Loads -2 to AC3\n\
        INC OL AC0,AC0 ; Load 3 to AC0\n";

        let mut ex = ExecutionContext::new();
        ex.load_initial_memory(assembler.assemble_lines(test_asm_line));
        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ex)?;
        instruction_decoder.decode_instructions(5)?;
        let ec = instruction_decoder.peek_ec();
        println!("{}", ec);
        assert_eq!(ec.ac[2], 1);
        assert_eq!(ec.ac[1], 0xffff_u16);
        assert_eq!(ec.ac[3], 0xfffe_u16);
        assert_eq!(ec.ac[0], 3);

        Ok(())
    }

    #[test]
    fn multiplication_by_add_and_shift_alc_instructions_test() -> std::io::Result<()> {


        let instruction_identifer = InstructionIdentifier::new()?;
        let assembler = Assembler::new_using_instruction_identifier(&instruction_identifer)?;

        // "014-000631 Nova Line Programmers reference series Jul79" kitabından alınmıştır

        let test_asm_line = "\
        LDFNW AC3 0xFFF0 ; 16 step for 16 bit \n\
        LDFNW AC1 0x3; multiplier operand  \n\
        LDFNW AC2 0x5; multiplicand operand \n\
        MOV R AC1,AC1 SNC; \n\
        MOV R AC0,AC0 SKP; \n\
        ADD ZR AC2,AC0 ; \n\
        INC AC3,AC3 SZR;\n\
        JMP -4\n\
        MOV CR AC1,AC1 ; Result is in AC1 \n\
        HALT\n";

        let mut ex = ExecutionContext::new();
        ex.load_initial_memory(assembler.assemble_lines(test_asm_line));
        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ex)?;
        instruction_decoder.decode_instructions(67)?;
        let ec = instruction_decoder.peek_ec();
        println!("{}", ec);

        assert_eq!(ec.ac[1], 15);


        Ok(())
    }

    #[test]
    fn unsigned_compare_alc_instructions_test() -> std::io::Result<()> {


        let instruction_identifer = InstructionIdentifier::new()?;
        let assembler = Assembler::new_using_instruction_identifier(&instruction_identifer)?;

        // "014-000631 Nova Line Programmers reference series Jul79" kitabından alınmıştır

        let test_asm_line = "\
        LDFNW AC0 0x1234\n\
        LDFNW AC1 0x1234\n\
        LDFNW AC2 0x0\n\
        SUB # AC0,AC1 SZR ; Equality check ( should skip )\n\
        HALT\n\
        SUB # AC0,AC1 SNR ; Unequality check ( shouldn't skip (increment AC2)) \n\
        INC AC2,AC2\n\
        LDFNW AC0 0x1 ; Comparison tests begin\n\
        LDFNW AC1 0x2\n\
        ADC Z# AC0,AC1 SNC ; Skip if ACS < ACD\n\
        HALT\n\
        SUB Z# AC0,AC1 SNC ; Skip if ACS <= ACD ( should skip (increment AC2))\n\
        HALT\n\
        INC AC2,AC2\n\
        LDFNW AC0 0x3\n\
        SUB Z# AC0,AC1 SZC ; Skip if ACS > ACD\n\
        HALT\n\
        INC AC2,AC2\n\
        ADC Z# AC0,AC1 SZC ; Skip if ACS >= ACD\n\
        HALT\n\
        INC AC2,AC2\n\
        HALT\n\
        ";

        let mut ex = ExecutionContext::new();
        ex.load_initial_memory(assembler.assemble_lines(test_asm_line));
        let mut instruction_decoder = instruction_decoder::InstructionDecoder::new_with_execution_context_default_for_tests(ex)?;
        instruction_decoder.decode_instructions(16)?;
        let ec = instruction_decoder.peek_ec();
        println!("{}", ec);

        assert_eq!(ec.ac[2], 4);




        Ok(())
    }




}
#[cfg(test)]
mod unimplemented_instruction_trap {
    use super::*;

    /// `155130` — the word INS64 group Q plants at Q02A (006057) and calls
    /// "AN UNIMP. INSTR. MOVZL# 2,3". An ALC with the no-load bit set and no skip field.
    const UNIMPLEMENTED_ALC: u16 = 0o155130;

    fn machine_with_a_handler_at(vector_word: u16) -> ExecutionContext {
        let mut ec = ExecutionContext::new();
        let mut mem = vec![0u16; 0o100];
        mem[0o43] = vector_word;
        ec.load_initial_memory(mem);
        ec.ip = 0o6057;
        ec
    }

    /// Section 2.29: "the address of the unimplemented instruction will be stored in location 42,
    /// and JMP to the address stored in location 43 will occur."
    ///
    /// Location 42 gets the address OF the instruction, not of the next one — the opposite of the
    /// Stack Overflow Trap. Group Q's handler does `ISZ 42` to step past it.
    #[test]
    fn an_unimplemented_alc_traps_instead_of_executing() {
        let mut ec = machine_with_a_handler_at(0o6046);
        ec.ac[3] = 0x1234;

        decode(UNIMPLEMENTED_ALC, Some(&mut ec));

        assert_eq!(ec.ip, 0o6046, "JMP to the address in location 43");
        assert_eq!(ec.mapping_unit.read_word_from_memory(0o42, true), 0o6057,
                   "location 42 holds the address OF the unimplemented instruction");
        assert_eq!(ec.ac[3], 0x1234, "the instruction must not execute");
    }

    /// Group Q builds its vector as `Q97 | 100000` at 006041 and stores that in location 43, so
    /// reaching the handler needs one level of indirection resolved (section 2.17: with expanded
    /// memory disabled, bit 0 is the indirect bit).
    #[test]
    fn the_trap_vector_resolves_indirection_when_expanded_memory_is_off() {
        let mut ec = machine_with_a_handler_at(0x8000 | 0o50);
        ec.mapping_unit.write_word_to_memory(0o50, 0o6046, true);

        decode(UNIMPLEMENTED_ALC, Some(&mut ec));

        assert_eq!(ec.ip, 0o6046, "one level of indirection followed");
    }

    /// With expanded memory enabled there is no indirect bit — all 16 bits are address.
    #[test]
    fn the_trap_vector_is_taken_whole_when_expanded_memory_is_on() {
        let mut ec = machine_with_a_handler_at(0x8000 | 0o50);
        ec.mapping_unit.msr.executive_expanded_memory = true;

        decode(UNIMPLEMENTED_ALC, Some(&mut ec));

        assert_eq!(ec.ip, 0x8000 | 0o50, "bit 0 is part of the address, not an indirect flag");
    }

    /// An ALC with the no-load bit but a real skip field is an ordinary compare, not a hole.
    #[test]
    fn a_no_load_alc_with_a_skip_field_is_not_a_trap() {
        let mut ec = machine_with_a_handler_at(0o6046);
        ec.ac[0] = 0;

        decode(0x850c, Some(&mut ec));   // SUB# AC0,AC0 SZR

        assert_ne!(ec.ip, 0o6046, "SUB# ... SZR is a normal instruction");
    }
}
