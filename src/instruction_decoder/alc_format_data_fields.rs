// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::fmt::{Display, Formatter};

#[derive(PartialEq,Eq)]
pub(crate) enum AlcFunctionField{
    COM,
    NEG,
    MOV,
    INC,
    ADC,
    SUB,
    ADD,
    AND
}

impl From<u8> for AlcFunctionField{
    fn from(value: u8) -> Self{
        match value {
            0 => AlcFunctionField::COM,
            1 => AlcFunctionField::NEG,
            2 => AlcFunctionField::MOV,
            3 => AlcFunctionField::INC,
            4 => AlcFunctionField::ADC,
            5 => AlcFunctionField::SUB,
            6 => AlcFunctionField::ADD,
            7 => AlcFunctionField::AND,
            _ => unreachable!()
        }
    }
}

impl Display for AlcFunctionField{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            AlcFunctionField::COM => "COM",
            AlcFunctionField::NEG => "NEG",
            AlcFunctionField::MOV => "MOV",
            AlcFunctionField::INC => "INC",
            AlcFunctionField::ADC => "ADC",
            AlcFunctionField::SUB => "SUB",
            AlcFunctionField::ADD => "ADD",
            AlcFunctionField::AND => "AND",
        };

        write!(f, "{}", str)
    }
}

#[derive(PartialEq,Eq)]
pub(crate) enum AlcCarryField{
    N,
    Z,
    O,
    C
}

impl From<u8> for AlcCarryField{
    fn from(value: u8) -> Self{
        match value {
            0 => AlcCarryField::N,
            1 => AlcCarryField::Z,
            2 => AlcCarryField::O,
            3 => AlcCarryField::C,
            _ => unreachable!()
        }
    }
}

impl Display for AlcCarryField{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            AlcCarryField::N => "",
            AlcCarryField::Z => "Z",
            AlcCarryField::O => "O",
            AlcCarryField::C => "C",
        };
        write!(f, "{}", str)
    }
}

#[derive(PartialEq,Eq)]
pub(crate) enum AlcShiftField{
    NoShift,
    L,
    R,
    S
}

impl From<u8> for AlcShiftField{
    fn from(value: u8) -> Self{
        match value {
            0 => AlcShiftField::NoShift,
            1 => AlcShiftField::L,
            2 => AlcShiftField::R,
            3 => AlcShiftField::S,
            _ => unreachable!()
        }
    }
}

impl Display for AlcShiftField{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            AlcShiftField::NoShift => "NoShift",
            AlcShiftField::L => "L",
            AlcShiftField::R => "R",
            AlcShiftField::S => "S",
        };

        write!(f,"{}",str)
    }
}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub(crate) enum Accumulators{
    AC0,
    AC1,
    AC2,
    AC3
}

impl From<u8> for Accumulators{
    fn from(value: u8) -> Self{
        match value {
            0 => Accumulators::AC0,
            1 => Accumulators::AC1,
            2 => Accumulators::AC2,
            3 => Accumulators::AC3,
            _ => unreachable!()
        }
    }
}

impl Into<usize> for Accumulators{
    fn into(self) -> usize{
        match self {
            Accumulators::AC0 => 0,
            Accumulators::AC1 => 1,
            Accumulators::AC2 => 2,
            Accumulators::AC3 => 3
        }
    }
}

impl Into<u8> for Accumulators{
    fn into(self) -> u8{
        match self {
            Accumulators::AC0 => 0,
            Accumulators::AC1 => 1,
            Accumulators::AC2 => 2,
            Accumulators::AC3 => 3
        }
    }
}

impl Display for Accumulators{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Accumulators::AC0 => "AC0",
            Accumulators::AC1 => "AC1",
            Accumulators::AC2 => "AC2",
            Accumulators::AC3 => "AC3",
        };

        write!(f, "{}", str)
    }
}

#[derive(Eq, PartialEq,Copy, Clone)]
pub(crate) enum AlcSkipField{
    NoSkip,
    SKP,
    SZC,
    SNC,
    SZR,
    SNR,
    SEZ,
    SBN
}

impl From<u8> for AlcSkipField{
    fn from(value: u8) -> Self {
        match value {
            0 => AlcSkipField::NoSkip,
            1 => AlcSkipField::SKP,
            2 => AlcSkipField::SZC,
            3 => AlcSkipField::SNC,
            4 => AlcSkipField::SZR,
            5 => AlcSkipField::SNR,
            6 => AlcSkipField::SEZ,
            7 => AlcSkipField::SBN,
            _ => unreachable!()
        }
    }
}

impl Display for AlcSkipField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            AlcSkipField::NoSkip => "No Skip",
            AlcSkipField::SKP => "SKP(always)",
            AlcSkipField::SZC => "SZC(skip if no carry)",
            AlcSkipField::SNC => "SNC(skip if carry)",
            AlcSkipField::SZR => "SZR(skip if zero)",
            AlcSkipField::SNR => "SNR(skip if non zero)",
            AlcSkipField::SEZ => "SEZ(Skip if either carry or zero)",
            AlcSkipField::SBN => "SBN(Skip if both carry and result are not zero)",
        };

        write!(f,"{}",str)
    }
}