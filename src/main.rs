mod chip_8;

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
        Some(100),
        None,
        Some(Chip8VMOptions {
            ..Default::default()
        }),
    );

    vm.load_rom_from_file("KALEID.ch8");
    // println!("{:?}", vm);

    vm.run();

    Ok(())
}
