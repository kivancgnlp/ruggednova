// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::fmt::{Display, Formatter};

#[derive(PartialEq,Eq)]
pub(crate) enum Transfer {
    NIO,
    DIA,
    DOA,
    DIB,
    DOB,
    DIC,
    DOC,
    SKP,
}

impl From<u8> for Transfer {
    fn from(value: u8) -> Transfer {
        match value {
            0 => Transfer::NIO,
            1 => Transfer::DIA,
            2 => Transfer::DOA,
            3 => Transfer::DIB,
            4 => Transfer::DOB,
            5 => Transfer::DIC,
            6 => Transfer::DOC,
            7 => Transfer::SKP,
            _ => unreachable!()
        }
    }
}

impl Display for Transfer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Transfer::NIO => write!(f, "NIO"),
            Transfer::DIA => write!(f, "DIA"),
            Transfer::DOA => write!(f, "DOA"),
            Transfer::DIB => write!(f, "DIB"),
            Transfer::DOB => write!(f, "DOB"),
            Transfer::DIC => write!(f, "DIC"),
            Transfer::DOC => write!(f, "DOC"),
            Transfer::SKP => write!(f, "SKP"),
        }
    }
}
impl Transfer {
    pub(crate) fn requires_data_transfer(&self) -> bool {
        match self {
            Transfer::NIO  | Transfer::SKP => false,
            _ => true,
        }

    }

    pub(crate) fn get_io_device_target_register(&self) -> u8 {
        match self {
            Transfer::DIA | Transfer::DOA => 0,
            Transfer::DIB | Transfer::DOB => 1,
            Transfer::DIC | Transfer::DOC => 2,
            _ => unreachable!()
        }
    }

    pub(crate) fn is_read(&self) -> bool {
        match self {
            Transfer::DIA | Transfer::DIB | Transfer::DIC => true,
            Transfer::DOA | Transfer::DOB | Transfer::DOC => false,
            _ => unreachable!()
        }
    }
}