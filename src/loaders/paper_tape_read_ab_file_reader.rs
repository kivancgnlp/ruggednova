use std::fmt::{Display, Formatter};
use std::io::{BufReader, Error, Read, Seek};
//use crate::loaders::paper_tape_read_ab_file_reader::AbBlock::{DataBlock, FillBlock, SkipBlock, StartBlock};
//use crate::loaders::{paper_tape_read_ab_file_reader};

enum AbBlock {

    DataBlock{
        length :u16,
        load_address : u16,
        data : Vec<u16>,
    },
    StartBlock{
        start_address : u16
    },

    FillBlock{
        start_address : u16,
        length : u16,
    },
    SkipBlock(),

}

impl Display for AbBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use AbBlock::*;
        match self {
            DataBlock{length, load_address, data} => write!(f, "DataBlock {{length: {:#x}, load_address: {:#x} - {:#x}}}", length, load_address, load_address + length),
            StartBlock{start_address} => write!(f, "StartBlock {{start_address: {:#x}}}", start_address),
            FillBlock{start_address, length} => write!(f, "FillBlock {{start_address: {:#x}, length: {:#x}}}", start_address, length),
            SkipBlock() => write!(f, "SkipBlock"),
        }
    }
}

pub(crate) fn paper_tape_image_loader(file_name : &str) -> Result<[u16;65536], Error> {

    let in_file = std::fs::File::open(file_name)?;
    let mut buf_reader = BufReader::new(in_file);

    let mut memory = [0_u16;65536];

    let mut block_number = 0_usize;

    use AbBlock::*;

    loop{
        let read_result = parse_block(&mut buf_reader);

        match read_result {
            Ok(ab_block) => {

                println!("Processing : {}", ab_block);
                match ab_block {
                    DataBlock {length, load_address, data}=> {
                        //println!("Processing data block {} for adr {:#x}, len : {}", block_number, load_address,length);

                        place_block_into_memory(load_address, &data, &mut memory);
                    },
                    _ => {
                        println!("Skipping block ");
                    }
                }
            }
            Err(e) => {
                println!("End of file reached. Error : {:?}", e);
                break;
            }
        }

        block_number += 1;


    }




    dump_memory(&memory);


    Ok(memory)


}

fn dump_memory( mem: &[u16; 65536]){

    for i in 0..1000{
        print!("{:#06x} ",mem[i]);
        if i%16 == 0{
            println!("");
        }
    }

}

fn place_block_into_memory(base:u16, data_block : &[u16], mem: &mut[u16]){



    for i in 0..data_block.len() {
        mem[base as usize + i] = data_block[i];
    }

}

fn read_le_word(input_stream: &mut impl Read) -> Result<u16, Error>{
    let mut word_bytes = [0u8;2];

    input_stream.read_exact(&mut word_bytes)?;
    Ok(u16::from_le_bytes(word_bytes))
}

fn read_single_byte(input_stream: &mut impl Read) -> Result<u8, Error>{
    let mut single_byte_buffer = [0u8;1];

    input_stream.read_exact(&mut single_byte_buffer)?;
    Ok(single_byte_buffer[0])
}


fn skip_null_bytes(input_stream: &mut (impl Read + Seek)) -> Result<(), Error>{

    let mut skipped_bytes: usize  = 0;

    while read_single_byte(input_stream)? == 0{
        skipped_bytes += 1;
    }

    println!("{} bytes skipped.", skipped_bytes);

    input_stream.seek(std::io::SeekFrom::Current(-1))?;

    Ok(())


}

fn parse_block(mut input_stream: impl Read + Seek) -> Result<AbBlock, Error> {

    skip_null_bytes(&mut input_stream)?;

    let word_count = read_le_word(&mut input_stream)?;

    let word_count_neg = (!word_count).wrapping_add(1);

    let address = read_le_word(&mut input_stream)?;

    let checksum = read_le_word(&mut input_stream)?;
    println!("word_count (original) : {:#x}", word_count);

    if word_count == 1 {
        return Ok(AbBlock::StartBlock{ start_address: address});
    }

    if word_count_neg <= 16{
        //println!("Data block with word count : {:#x} and load address : {:#x}", word_count_neg,address);

        let mut segment_words : Vec<u16> = Vec::with_capacity(word_count_neg as usize);

        for i in 0..word_count_neg {
            let word = read_le_word(&mut input_stream)?;
            //println!("word : {:#x}", word);
            segment_words.push(word);
        }

        return Ok(AbBlock::DataBlock{length: word_count_neg, load_address: address, data: segment_words});
    }

    if word_count_neg > 16{
        return Ok(AbBlock::FillBlock{start_address: address, length: word_count_neg});
    }

    !todo!("Skip block?");


}


#[cfg(test)]
mod tests {
    use std::io::{BufWriter, Write};
    use super::*;
    #[test]
    #[ignore]
    fn test_ab_reader() -> Result<(), Error>  {
        paper_tape_image_loader("Data/Diagnostic images/095-000005-01__Nova_Logic_Test__1969.ab")?;
        Ok(())
    }


    #[test]
    #[ignore]
    fn dump_ab_to_file() -> Result<(), Error>  {
        let mem = paper_tape_image_loader("Data/Diagnostic images/095-000005-01__Nova_Logic_Test__1969.ab")?;

        let mut file = std::fs::File::create("dump_ab_to_file.bin")?;
        let mut buff_wr = BufWriter::new(file);

        mem.iter().for_each(|x| {
            let by = x.to_be_bytes();
            buff_wr.write_all(&by).expect("Error writing data");
        });


        Ok(())
    }


}