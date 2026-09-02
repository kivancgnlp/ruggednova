// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

/// Sets a specific bit in a 16-bit unsigned integer.
///
/// This function sets the bit at the specified `bit_index` to `1` in the
/// provided mutable `input_value`. It treats the bit positions as zero-indexed,
/// starting from the most significant bit (MSB).
///
/// For example:
/// - `bit_index = 0` sets the MSB.
/// - `bit_index = 15` sets the least significant bit (LSB).
///
/// # Arguments
///
/// - `input_value`: A mutable reference to a `u16` integer whose bit will be set.
/// - `bit_index`: An unsigned 8-bit integer specifying which bit to set (valid range is 0–15).
///
/// # Panics
///
/// This function will panic if `bit_index` is greater than or equal to 16.
///
/// # Example
///
/// ```rust
/// let mut number: u16 = 0b0000_0000_0000_0000;
/// set_bit(&mut number, 0); // Sets the MSB
/// assert_eq!(number, 0b1000_0000_0000_0000);
///
/// set_bit(&mut number, 15); // Sets the LSB
/// assert_eq!(number, 0b1000_0000_0000_0001);
/// ```
///
/// # Note
///
/// The internal representation in memory is little-endian, but this
/// function interprets the bit indices with a "big-endian" interpretation
/// (MSB is index 0, LSB is index 15).

pub(super) fn set_bit(input_value: &mut u16, bit_index:u8) {
    assert!(bit_index < 16);
    let bit_index_inverted = 15 - bit_index;
    let bit_mask = 1 << bit_index_inverted;
    *input_value |= bit_mask;
}

fn get_bit_mask(bit_count: u8) -> u16 {
    let mask_bits = 0xffffu16;

    let mask_bit_count_inverted = 16 - bit_count;

    mask_bits.unbounded_shr(mask_bit_count_inverted as u32) // Burda 0 bit istendiği özel durumda overflow olmaması için unbounded shift kullanıldı

}

/// Extracts a subset of bits from a 16-bit unsigned integer.
///
/// This function retrieves a range of bits from the input 16-bit unsigned integer (`input_value`)
/// starting from the specified `start_index` and ending at the `end_index`. The output is a
/// new 16-bit unsigned integer where the selected bits are aligned to the least significant bit,
/// and all other bits are zeroed out.
///
/// # Parameters
/// - `input_value`: A 16-bit unsigned integer from which the bits are extracted.
/// - `start_index`: The starting bit index (0-based, inclusive). Must be less than 16.
/// - `end_index`: The ending bit index (0-based, inclusive). Must be less than 16
///   and greater than or equal to `start_index`.
///
/// # Returns
/// A 16-bit unsigned integer containing the extracted bits aligned to the least significant
/// bit position. All bits outside the specified range are set to zero.
///
/// # Panics
/// The function will panic if any of the following conditions are true:
/// - `start_index` is 16 or greater.
/// - `end_index` is 16 or greater.
/// - `start_index` is greater than `end_index`.
///
/// # Examples
/// ```
/// let input = 0b1011_1101_0110_1111_u16;
/// let start_index = 4;
/// let end_index = 8;
///
/// let extracted_bits = get_bits(input, start_index, end_index);
/// assert_eq!(extracted_bits, 0b0000_0000_0000_1101_u16);
/// ```
///
/// The example above extracts bits 4 through 8 from `input`, resulting in the value `0b1101`.
///
/// # Note
/// This function internally uses a helper function `get_bit_mask` to generate a bitmask
/// based on the number of bits to be extracted. Ensure that `get_bit_mask` is correctly implemented
/// and available within the scope of this function.
pub(crate) fn get_bits(input_value:u16, start_index:u8, end_index:u8) -> u16 {
    assert!(start_index < 16);
    assert!(end_index < 16);
    assert!(start_index <= end_index);

    let end_index_inverted = 15 - end_index;

    let shifted_value = input_value >> end_index_inverted;
    let mask_bit_count = end_index - start_index + 1;
    let mask_bits = get_bit_mask(mask_bit_count);


    shifted_value & mask_bits
}

pub(crate) fn set_bits(input_value:&mut u16, start_index:u8, end_index:u8, value_to_set: u16) {
    assert!(start_index < 16);
    assert!(end_index < 16);
    assert!(start_index <= end_index);

    let mask_bit_count = end_index - start_index + 1;
    let mask_bits = get_bit_mask(mask_bit_count);

    let value_masked = value_to_set & mask_bits;
    let end_index_inverted = 15 - end_index;
    let shifted_value = value_masked << end_index_inverted;
    let mask_shifted = mask_bits << end_index_inverted;
    let mask_shifted_and_inverted = mask_shifted ^ 0xffff_u16;


    *input_value &= mask_shifted_and_inverted;

    *input_value |= shifted_value;

}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::bit_utils::*;

    #[test]
    fn test_01(){
        let mut test_val = 0u16;
        set_bit(&mut test_val, 0);
        set_bit(&mut test_val, 1);
        set_bit(&mut test_val, 2);

        assert_eq!(get_bits(test_val, 0, 2), 7);
        assert_eq!(get_bits(test_val, 0, 0), 1);
    }

    #[test]
    fn test_02(){
        let mut test_val = 0u16;
        set_bits(&mut test_val,0,2,3);


        assert_eq!(get_bits(test_val, 0, 2), 3);
        assert_eq!(get_bits(test_val, 0, 0), 0);

        test_val = 0xffffu16;
        set_bits(&mut test_val,8,15,0x81); // Set by clearing test
        assert_eq!(test_val, 0xff81);
        set_bits(&mut test_val,0,0,0);
        assert_eq!(test_val, 0x7f81);
    }

    #[test]
    fn get_bit_test(){
        let test_val = 0x8001_u16;

        assert_eq!(get_bits(test_val, 0, 0), 1);
        assert_eq!(get_bits(test_val, 15, 15), 1);
    }

    #[test]
    fn set_bit_test(){
        let mut test_val = 0_u16;

        set_bits(&mut test_val,0,0,0x0ff);
        set_bits(&mut test_val,15,15,0x0ff);

        assert_eq!(get_bits(test_val, 0, 0), 1);
        assert_eq!(get_bits(test_val, 15, 15), 1);
        assert_eq!(test_val, 0x8001_u16);
    }

    #[test]
    fn get_bitmask_test(){
        for bit_count in 0..=16{
            let mask = get_bit_mask(bit_count);
            assert_eq!(u16::count_ones(mask) as u8, bit_count);
        }
    }
}