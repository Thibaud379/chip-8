use chip_8::*;
fn main() -> std::io::Result<()> {
    // let mut vm = Chip8VM::new(
    //     Some(10),
    //     None,
    //     Some(Chip8VMOptions {
    //         debug: true,
    //         // hide_display: trcue,
    //         keep_display: true,
    //         new_jump_off: true,
    //         ..Default::default()
    //     }),
    // );
    let mut vm = Chip8VM::new(
        Some(12),
        None,
        Some(Chip8VMOptions {
            keep_display: true,
            hide_display: true,
            ..Default::default()
        }),
    );

    vm.load_rom_from_file("ibm.ch8");
    println!("{:?}", vm);

    vm.run();

    Ok(())
}
