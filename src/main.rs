mod chip_8;

use std::{
    fs::File,
    io::{BufReader, Read},
};

use chip_8::*;
fn main() -> std::io::Result<()> {
    // let mut vm = Chip8VM::new(None, None, Some(Chip8VMOptions { debug_ram: true }));
    let mut vm = Chip8VM::new(None, None, None);
    let f = File::open("ibm.ch8")?;
    let mut reader = BufReader::new(f);
    let mut buf: Vec<u8> = Vec::new();

    reader.read_to_end(&mut buf)?;

    vm.load_rom(&buf);
    // println!("{:?}", vm);

    vm.run();

    Ok(())
}
