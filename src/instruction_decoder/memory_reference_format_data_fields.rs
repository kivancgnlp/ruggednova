// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::fmt::{Display, Formatter};

#[derive(PartialEq,Eq,Clone,Copy,Debug)]
pub(crate) enum ReferenceType{
    Page0,
    PcRelative,
    AC2Based,
    AC3Based
}

impl From<u8> for ReferenceType{
    fn from(value: u8) -> Self{
        match value{
            0 => ReferenceType::Page0,
            1 => ReferenceType::PcRelative,
            2 => ReferenceType::AC2Based,
            3 => ReferenceType::AC3Based,
            _ => unreachable!()
        }
    }
}


#[derive(PartialEq,Eq)]
pub(crate) enum JmpFunction{
    JMP,
    JSR,
    ISZ,
    DSZ
}

impl From<u8> for JmpFunction {
    fn from(instruction_word:u8) -> Self {
        match instruction_word {
            0 => JmpFunction::JMP,
            1 => JmpFunction::JSR,
            2 => JmpFunction::ISZ,
            3 => JmpFunction::DSZ,
            _ => unreachable!()
        }
    }
}

impl Display for JmpFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result{
        match self {
            JmpFunction::JMP => write!(f, "JMP"),
            JmpFunction::JSR => write!(f, "JSR"),
            JmpFunction::ISZ => write!(f, "ISZ"),
            JmpFunction::DSZ => write!(f, "DSZ")
        }
    }
}