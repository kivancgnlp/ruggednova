// SPDX-License-Identifier: LicenseRef-Proprietary
// Copyright (c) 2026 Kivanc Gunalp. All rights reserved.

use std::str::FromStr;
use crate::instruction_decoder::alc_format_data_fields::Accumulators;

pub(crate) fn parse_accumulator(ac_str : &str) -> Option<Accumulators>{
    let ac_index = ac_str.find("AC")?;
    let ac_no_str = ac_str.get(ac_index+2..ac_str.len())?;
    let id = u8::from_str(ac_no_str).ok()?;
    Some(Accumulators::from(id))
}

pub(crate) fn parse_accumulators(acs_and_acd_str : &str) ->Option<(Accumulators, Accumulators)>{

    if !acs_and_acd_str.contains(",") {
        eprintln!("acs_and_acd_str does not contain comma separated values");
        return None;
    }

    let mut third_part_splitter = acs_and_acd_str.split(',');
    let mut accumulators: [Accumulators;2] = [Accumulators::AC0,Accumulators::AC0];


    for i in 0..2{
        let ac_str = third_part_splitter.next()?;
        accumulators[i] = parse_accumulator(ac_str.trim())?;

    }

    Some((accumulators[0].clone(), accumulators[1].clone()))

}

#[cfg(test)]
mod tests {
    use crate::instruction_decoder::alc_format_data_fields::Accumulators;

    #[test]
    fn test_parse_accumulators() -> Result<(), std::io::Error> {
        use crate::assembler::common_assembler_utils::parse_accumulators;

        let acs_acd_str = "AC1,AC2";

        let result = parse_accumulators(acs_acd_str).ok_or(std::io::Error::other(""))?;

        assert_eq!(result, (Accumulators::AC1,Accumulators::AC2));
        println!("{} {}",result.0,result.1);

        let acs_acd_str = "AC0 , AC1";
        let result = parse_accumulators(acs_acd_str).ok_or(std::io::Error::other(""))?;

        assert_eq!(result, (Accumulators::AC0,Accumulators::AC1));
        println!("{} {}",result.0,result.1);
        Ok(())
    }
}