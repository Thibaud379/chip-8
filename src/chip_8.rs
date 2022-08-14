#[allow(dead_code)]
use std::{thread::sleep, time::Duration};

const FONT_SIZE: usize = 80;
type Font = [u8; FONT_SIZE];

const FONT_START: usize = 0x50;
const FONT: [u8; FONT_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const FREQ: u32 = 700;

const RAM_SIZE: usize = 4096;
type Ram = [u8; RAM_SIZE];

const RAM_ROM_START: usize = 0x200;

const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;
type Display = [[bool; DISPLAY_WIDTH]; DISPLAY_HEIGHT];

const DISPLAY_EMPTY: Display = [[false; DISPLAY_WIDTH]; DISPLAY_HEIGHT];

#[derive(Default)]
struct Registers {
    pc: U12,
    i: U12,
    v0: u8,
    v1: u8,
    v2: u8,
    v3: u8,
    v4: u8,
    v5: u8,
    v6: u8,
    v7: u8,
    v8: u8,
    v9: u8,
    va: u8,
    vb: u8,
    vc: u8,
    vd: u8,
    ve: u8,
    vf: u8,
}
impl Registers {
    fn set(&mut self, reg: U4, value: u8) {
        *self.get_reg(reg) = value;
    }

    fn get(&mut self, reg: U4) -> u8 {
        *self.get_reg(reg)
    }

    fn get_reg(&mut self, reg: U4) -> &mut u8 {
        //Check that we have a valid register number
        assert!(reg >> 4 == 0);
        match reg & 0xF {
            0 => &mut self.v0,
            1 => &mut self.v1,
            2 => &mut self.v2,
            3 => &mut self.v3,
            4 => &mut self.v4,
            5 => &mut self.v5,
            6 => &mut self.v6,
            7 => &mut self.v7,
            8 => &mut self.v8,
            9 => &mut self.v9,
            0xA => &mut self.va,
            0xB => &mut self.vb,
            0xC => &mut self.vc,
            0xD => &mut self.vd,
            0xE => &mut self.ve,
            0xF => &mut self.vf,
            _ => panic!("Cannot happen"),
        }
    }
}

impl std::fmt::Debug for Registers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "---  Registers ---")?;
        write!(f, "pc|")?;
        writeln!(f, " i|")?;
        write!(f, "{:>2x}|", self.pc)?;
        writeln!(f, "{:>2x}|", self.i)?;
        for i in 0..15 {
            write!(f, "v{:x}|", i)?;
        }
        writeln!(f, "vf")?;
        write!(f, "{:>2x}|", self.v0)?;
        write!(f, "{:>2x}|", self.v1)?;
        write!(f, "{:>2x}|", self.v2)?;
        write!(f, "{:>2x}|", self.v3)?;
        write!(f, "{:>2x}|", self.v4)?;
        write!(f, "{:>2x}|", self.v5)?;
        write!(f, "{:>2x}|", self.v6)?;
        write!(f, "{:>2x}|", self.v7)?;
        write!(f, "{:>2x}|", self.v8)?;
        write!(f, "{:>2x}|", self.v9)?;
        write!(f, "{:>2x}|", self.va)?;
        write!(f, "{:>2x}|", self.vb)?;
        write!(f, "{:>2x}|", self.vc)?;
        write!(f, "{:>2x}|", self.vd)?;
        write!(f, "{:>2x}|", self.ve)?;
        write!(f, "{:>2x}", self.vf)
    }
}

#[derive(Default, Debug)]
#[allow(dead_code)]
struct Timers {
    delay: u8,
    sound: u8,
}

type U4 = u8;
type U12 = u16;
#[derive(Debug, PartialEq)]
enum Chip8Instr {
    Clear,
    Jump(U12),
    Set(U4, u8),
    Add(U4, u8),
    SetI(U12),
    Display(U4, U4, U4),
    Return,
    Unknow,
}

impl From<u16> for Chip8Instr {
    fn from(input: u16) -> Self {
        match input >> 12 {
            0 => {
                if input & 0xF == 0xE {
                    Self::Return
                } else {
                    Self::Clear
                }
            }
            1 => Self::Jump(input & 0xFFF),
            6 => Self::Set((input >> 8 & 0xF) as U4, (input & 0xFF) as u8),
            7 => Self::Add((input >> 8 & 0xF) as U4, (input & 0xFF) as u8),
            0xA => Self::SetI(input & 0xFFF),
            0xD => Self::Display(
                (input >> 8 & 0xF) as U4,
                (input >> 4 & 0xF) as U4,
                (input & 0xF) as U4,
            ),
            _ => Self::Unknow,
        }
    }
}

#[derive(Default)]
pub struct Chip8VMOptions {
    pub debug_ram: bool,
}

pub struct Chip8VM {
    // 4kB of memory
    ram: Ram,

    // Display of 64*32 pixels (On or Off)
    display: Display,

    //All registers
    registers: Registers,

    //Timers
    timers: Timers,

    //Stack
    stack: Vec<U12>,

    //Clock speed (Hz)
    #[allow(dead_code)]
    freq: u32,

    //Misc options
    options: Chip8VMOptions,
}

const LINE_WIDTH: usize = 32;
const LINES: usize = RAM_SIZE / LINE_WIDTH;

impl std::fmt::Debug for Chip8VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // writeln!(f, "{:?}", self.timers)?;
        writeln!(f, "{:?}", self.registers)?;
        // writeln!(f, "--- Stack ---\n{:?}", self.stack)?;
        let r = writeln!(
            f,
            "Next instruction: {:?}",
            Chip8Instr::from(self.fetch_instruction())
        );
        if self.options.debug_ram {
            writeln!(f, "--- RAM dump ---")?;
            for i in 0..LINES - 1 {
                writeln!(
                    f,
                    "{:>2x?}",
                    &self.ram[i * LINE_WIDTH..(i + 1) * LINE_WIDTH]
                )?;
            }
            write!(f, "{:>2x?}", &self.ram[(LINES - 1) * LINE_WIDTH..])
        } else {
            r
        }
    }
}

impl std::fmt::Display for Chip8VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:#?}", self.display)
    }
}

impl Chip8VM {
    pub fn new(freq: Option<u32>, font: Option<Font>, options: Option<Chip8VMOptions>) -> Self {
        Chip8VM {
            ram: Chip8VM::init_ram(font.unwrap_or(FONT)),
            display: DISPLAY_EMPTY,
            registers: Registers {
                pc: U12::try_from(RAM_ROM_START).expect("RAM_ROM_START is small enough"),
                ..Registers::default()
            },
            timers: Timers::default(),
            stack: Vec::new(),
            freq: freq.unwrap_or(FREQ),
            options: options.unwrap_or_default(),
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        assert!(
            rom.len() <= RAM_SIZE - RAM_ROM_START,
            "Rom to big: {}B for {}B available",
            rom.len(),
            RAM_SIZE - RAM_ROM_START
        );
        for index in RAM_ROM_START..RAM_ROM_START + rom.len() {
            self.ram[index] = rom[index - RAM_ROM_START];
        }
    }

    pub fn run(&mut self) {
        loop {
            let instruction = self.fetch_instruction();
            println!("input: {:x}", instruction);
            let instruction = Chip8Instr::from(instruction);
            println!("decoded: {:?}", instruction);
            self.incr_pc();
            self.execute(instruction);
            println!("{:?}", &self);
            self.display();
        }
    }

    fn execute(&mut self, instruction: Chip8Instr) {
        match instruction {
            Chip8Instr::Clear => self.display = DISPLAY_EMPTY,
            Chip8Instr::Jump(nnn) => self.registers.pc = nnn,
            Chip8Instr::Set(vx, nn) => self.registers.set(vx, nn),
            Chip8Instr::Add(vx, nn) => {
                let curr = self.registers.get(vx);
                self.registers.set(vx, nn + curr);
            }
            Chip8Instr::SetI(nnn) => self.registers.i = nnn,
            Chip8Instr::Display(vx, vy, n) => {
                let x = self.registers.get(vx); //% (DISPLAY_WIDTH as u8);
                let y = self.registers.get(vy); //% (DISPLAY_HEIGHT as u8);
                let sprite_addr = self.registers.i;
                let sprite_height = n;
                self.draw_sprite(x, y, sprite_addr, sprite_height);
            }
            _ => panic!("Not implemented ({:?})", instruction),
        }
    }

    fn fetch_instruction(&self) -> u16 {
        let first_byte = self.ram[self.registers.pc as usize];
        let second_byte = self.ram[(self.registers.pc + 1) as usize];
        let both_bytes = u16::from_be_bytes([first_byte, second_byte]);
        both_bytes
    }

    fn display(&self) {
        sleep(Duration::from_millis(500));
        // print!("{esc}c", esc = 27 as char);
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                if self.display[y][x] {
                    print!("⬜");
                } else {
                    print!("⬛");
                }
            }
            println!();
        }
    }

    fn draw_sprite(&mut self, x: u8, y: u8, sprite_addr: U12, sprite_height: U4) {
        let sprite_data =
            &self.ram[sprite_addr as usize..(sprite_addr + sprite_height as u16) as usize];
        for curr_y in y..y + sprite_height {
            // if curr_y >= DISPLAY_HEIGHT as u8 {
            //     break;
            // }
            for curr_x in x..x + 8 {
                // if curr_x >= DISPLAY_WIDTH as u8 {
                //     break;
                // }

                self.display[curr_y as usize][x as usize + 8 - (curr_x as usize - x as usize)] ^=
                    ((sprite_data[(curr_y - y) as usize] >> (curr_x - x)) & 1) == 1
            }
        }
    }

    fn incr_pc(&mut self) {
        self.registers.pc += 2;
    }
    fn init_ram(font: Font) -> Ram {
        let mut ram = [0; RAM_SIZE];
        for index in FONT_START..FONT_START + FONT_SIZE {
            ram[index] = font[index - FONT_START];
        }
        ram
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_new() {
        let vm = Chip8VM::new(None, None, None);
        assert_eq!(vm.ram[FONT_START..FONT_START + FONT_SIZE], FONT);
        assert_eq!(vm.freq, 700);
    }

    #[test]
    fn load_empty() {
        let mut vm = Chip8VM::new(None, None, None);
        let before = vm.ram.clone();
        vm.load_rom(&[]);
        assert_eq!(before, vm.ram);
    }
    #[test]
    #[should_panic]
    fn load_too_big() {
        let mut vm = Chip8VM::new(None, None, None);
        vm.load_rom(&[1; 4096 - 511]);
    }

    #[test]
    fn load_fit() {
        let mut vm = Chip8VM::new(None, None, None);
        vm.load_rom(&[1; 4096 - 512]);
    }

    #[test]
    fn parse_instructions() {
        let tests: Vec<(u16, Chip8Instr)> = vec![
            (0x00E0, Chip8Instr::Clear),
            (0x00EE, Chip8Instr::Return),
            (0x1245, Chip8Instr::Jump(0x245)),
            (0x1EF3, Chip8Instr::Jump(0xEF3)),
            (0x6336, Chip8Instr::Set(0x3, 0x36)),
            (0x6F4A, Chip8Instr::Set(0xF, 0x4A)),
            (0x7336, Chip8Instr::Add(0x3, 0x36)),
            (0x7F4A, Chip8Instr::Add(0xF, 0x4A)),
            (0xA6BA, Chip8Instr::SetI(0x6BA)),
            (0xD6FA, Chip8Instr::Display(0x6, 0xF, 0xA)),
            (0xD3BA, Chip8Instr::Display(0x3, 0xB, 0xA)),
            (0xDC2A, Chip8Instr::Display(0xC, 0x2, 0xA)),
            (0xd01f, Chip8Instr::Display(0, 1, 15)),
        ];

        tests
            .iter()
            .for_each(|(i, r)| assert_eq!(Chip8Instr::from(*i), *r));
    }
}
