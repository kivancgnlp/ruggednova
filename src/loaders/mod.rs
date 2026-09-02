use std::io::{BufReader, Error, Read};

struct DataBlock{
    load_address: u16,
    word_count: u16,
    data: Vec<u16>,
}

fn parse_data_block(mut input_stream: impl Read) -> Result<DataBlock, Error> {

    let mut word = read_le_word(&mut input_stream)?;

    while word == 0 {
        word = read_le_word(&mut input_stream)?;

    }

    word = !word;
    word += 1;

    let word_count = word;
    println!("Word count : {:#x}", word_count);
    let mut segment_words : Vec<u16> = Vec::with_capacity(word_count as usize);

    let load_address = read_le_word(&mut input_stream)?;
    println!("load adres : {:#x}", load_address);

    word = read_le_word(&mut input_stream)?;
    println!("checksum : {:#x}", word);

    for i in 0..word_count {
        word = read_le_word(&mut input_stream)?;
        //println!("word : {:#x}", word);
        segment_words.push(word);
    }

    Ok(DataBlock{data: segment_words, load_address, word_count })
}

pub(crate) fn paper_tape_image_loader(file_name : &str) -> Result<[u16;65536], Error> {

    let in_file = std::fs::File::open(file_name)?;
    let mut buf_reader = BufReader::new(in_file);

    let mut memory = [0_u16;65536];

    let mut read_result = parse_data_block(&mut buf_reader);

    let mut block_number = 0_usize;
    while read_result.is_ok() {
        block_number += 1;
        place_block_into_memory(read_result?, &mut memory);
        println!("Proceesin block : {}",block_number);
        read_result = parse_data_block(&mut buf_reader);
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

fn place_block_into_memory(data_block: DataBlock, mem: &mut[u16; 65536]){

    let base = data_block.load_address as usize;

    for i in 0..data_block.word_count as usize {
        mem[base + i] = data_block.data[i];
    }

}

fn read_le_word(input_stream: &mut impl Read) -> Result<u16, Error>{
    let mut word_bytes = [0u8;2];

    input_stream.read_exact(&mut word_bytes)?;
    Ok(u16::from_le_bytes(word_bytes))
}



#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore]
    fn test_ab_reader() -> Result<(), Error>  {
        paper_tape_image_loader("Data/Diagnostic images/095-000005-01__Nova_Logic_Test__1969.ab")?;
        Ok(())
    }


}