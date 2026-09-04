// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use crate::instruction_decoder::alc_format_data_fields::Accumulators;
use crate::virtual_machine::ExecutionContext;

pub(crate) fn emulate_io_device(io_device: u8, io_device_register: u8, is_read: bool, acc : Accumulators, ec: &mut ExecutionContext, dev_name: Option<&str>) {

    let acc_id = acc as usize;
    let mut handled = false;

   match io_device {

       0o14 => {
           ec.rtc_initialized = true;
           println!("RTC initialized");
           handled = true;
       }

       0o11 => {

           let ch = ec.ac[acc_id] as u8;
           println!("TTO output : {:#x} : {}", ch as u8, ch as char);
           ec.tto_buffer.push(ch as char);
           handled = true;
       }
      

       _ => {
           handled = false;

       }
   }

    if !handled {
        let dir = if is_read {"Read from device"} else {"Write to device"};
        let io_dev_info = if let Some(dev_name) = dev_name {
            format!("io device: {:#o} ({})", io_device, dev_name)
        }else {
            format!("io device: {:#o}", io_device)
        };

        let str = format!("Emulation not implemented for {}, direction : {}, target device reg : {}, data for write {:#x}", io_dev_info,dir,io_device_register, ec.ac[acc_id]);
        writeln!(ec.mapping_unit.log_writer.borrow_mut(), "{}", str.as_str());

        //eprintln!("Emulation not implemented for {}, direction : {}, target device reg : {}, data for write {:#x}", io_dev_info,dir,io_device_register, ec.ac[acc_id]);
        //ec.ac[acc_id] = 0; // Default response

    }

}

