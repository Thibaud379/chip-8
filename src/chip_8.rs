use std::fmt::Debug;
use std::io::Read;
use std::sync::{Arc, Condvar, Mutex};
use std::{thread, time::Duration, time::Instant};

type Ram = [u8; Chip8VM::RAM_SIZE];
type Font = [u8; Chip8VM::FONT_SIZE];
type Display = [[bool; Chip8VM::DISPLAY_WIDTH]; Chip8VM::DISPLAY_HEIGHT];
type U4 = u8;
type U12 = u16;

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

    fn get(&self, reg: U4) -> u8 {
        assert!(reg >> 4 == 0, "{reg} was out of bounds");
        match reg & 0xF {
            0 => self.v0,
            1 => self.v1,
            2 => self.v2,
            3 => self.v3,
            4 => self.v4,
            5 => self.v5,
            6 => self.v6,
            7 => self.v7,
            8 => self.v8,
            9 => self.v9,
            0xA => self.va,
            0xB => self.vb,
            0xC => self.vc,
            0xD => self.vd,
            0xE => self.ve,
            0xF => self.vf,
            _ => panic!("Cannot happen"),
        }
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
        writeln!(f, "---  Registers  ---")?;
        write!(f, " pc|")?;
        writeln!(f, "  i|")?;
        write!(f, "{:>3x}|", self.pc)?;
        writeln!(f, "{:>3x}|", self.i)?;
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
struct Timers {
    delay: u8,
    buzzer: u8,
}
impl Timers {
    const TIMER_FREQ: u32 = 60;

    fn update(&mut self) {
        self.delay = self.delay.saturating_sub(1);
        self.buzzer = self.buzzer.saturating_sub(1);
    }
}

struct TimersWrapper {
    timers: Arc<Mutex<Timers>>,
    lock: Arc<(Mutex<bool>, Condvar)>,
}
impl Debug for TimersWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "---  Timers  ---")?;
        let t = self.timers.lock().unwrap();
        writeln!(f, "delay: {} | buzzer: {}", t.delay, t.buzzer)
    }
}
impl TimersWrapper {
    fn new() -> Self {
        let wrapper = TimersWrapper {
            timers: Arc::new(Mutex::new(Timers {
                delay: 120,
                buzzer: 0,
            })),
            lock: Arc::new((Mutex::new(false), Condvar::new())),
        };
        let t_clone = Arc::clone(&wrapper.timers);
        let l_clone = Arc::clone(&wrapper.lock);
        thread::spawn(move || {
            let timer = t_clone;
            let (lock, cvar) = &*l_clone;
            let mut last_update = Instant::now();
            let mut wait: f64;
            loop {
                //wait for start signal
                let mut started = lock.lock().unwrap();
                // As long as the value inside the `Mutex<bool>` is `false`, we wait.
                while !*started {
                    println!("Timers waiting to start");
                    started = cvar.wait(started).unwrap();
                    println!("Timers starting");
                }

                wait =
                    (1. / Timers::TIMER_FREQ as f64 - last_update.elapsed().as_secs_f64()).max(0.);

                thread::sleep(Duration::from_secs_f64(wait));
                timer.lock().unwrap().update();
                last_update = Instant::now();
            }
        });
        wrapper
    }
}

#[derive(Debug, PartialEq)]
enum Chip8Instr {
    Clear,
    Return,
    Jump(U12),
    Call(U12),
    IfNE(U4, u8),
    IfE(U4, u8),
    IfRNE(U4, U4),
    Set(U4, u8),
    Add(U4, u8),
    SetR(U4, U4),
    BitOp(U4, U4, U4),
    ArithmOp(U4, U4, U4),
    ShiftOp(U4, U4, U4),
    IfRE(U4, U4),
    SetI(U12),
    JumpOff(U12),
    Rand(U4, u8),
    Display(U4, U4, U4),
    KeyUp(U4),
    KeyDown(U4),
    GetDelay(U4),
    GetKey(U4),
    SetDelay(U4),
    SetBuzzer(U4),
    IncrI(U4),
    Char(U4),
    Decimal(U4),
    Save(U4),
    Load(U4),
    Unknown,
}
impl From<u16> for Chip8Instr {
    fn from(input: u16) -> Self {
        let x = (input >> 8 & 0xF) as U4;
        let y = (input >> 4 & 0xF) as U4;
        let n = (input & 0xF) as U4;
        let nn = (input & 0xFF) as u8;
        let nnn = (input & 0xFFF) as U12;
        match input >> 12 {
            0 if n == 0xE => Self::Return,
            0 => Self::Clear,
            1 => Self::Jump(nnn),
            2 => Self::Call(nnn),
            3 => Self::IfNE(x, nn),
            4 => Self::IfE(x, nn),
            5 => Self::IfRNE(x, y),
            6 => Self::Set(x, nn),
            7 => Self::Add(x, nn),
            8 if n == 0 => Self::SetR(x, y),
            8 if n < 4 => Self::BitOp(x, y, n),
            8 if n == 6 || n == 0xE => Self::ShiftOp(x, y, n),
            8 => Self::ArithmOp(x, y, n),
            9 => Self::IfRE(x, y),
            0xA => Self::SetI(nnn),
            0xB => Self::JumpOff(nnn),
            0xC => Self::Rand(x, nn),
            0xD => Self::Display(x, y, n),
            0xE if n == 0xE => Self::KeyUp(x),
            0xE => Self::KeyDown(x),
            0xF if nn == 0x07 => Self::GetDelay(x),
            0xF if nn == 0x0A => Self::GetKey(x),
            0xF if nn == 0x15 => Self::SetDelay(x),
            0xF if nn == 0x18 => Self::SetBuzzer(x),
            0xF if nn == 0x1E => Self::IncrI(x),
            0xF if nn == 0x29 => Self::Char(x),
            0xF if nn == 0x33 => Self::Decimal(x),
            0xF if nn == 0x55 => Self::Save(x),
            0xF if nn == 0x65 => Self::Load(x),
            _ => Self::Unknown,
        }
    }
}

#[derive(Default)]
pub struct Chip8VMOptions {
    //Debug/Output options
    pub hide_display: bool,
    pub debug: bool,
    pub debug_ram: bool,
    pub keep_display: bool,

    //Ambiguous instructions toggle
    pub incr_i_when_mem: bool,
    pub new_jump_off: bool,
    pub old_shift: bool,
}

pub struct Chip8VM {
    // 4kB of memory
    ram: Ram,

    // Display of 64*32 pixels (On or Off)
    pub display: Display,

    //All registers
    registers: Registers,

    //Timers
    timers: TimersWrapper,

    //Stack
    stack: Vec<U12>,

    //Clock speed (Hz)
    pub freq: u32,

    //Misc options
    options: Chip8VMOptions,
}
impl std::fmt::Debug for Chip8VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self.timers)?;
        writeln!(f, "{:?}", self.registers)?;
        // writeln!(f, "--- Stack ---\n{:?}", self.stack)?;
        let r = writeln!(
            f,
            "Next instruction: {:?}",
            Chip8Instr::from(self.fetch_instruction())
        );
        if self.options.debug_ram {
            writeln!(f, "--- RAM dump ---")?;
            for i in 0..Self::RAM_DISP_LINES - 1 {
                writeln!(
                    f,
                    "{:>2x?}",
                    &self.ram[i * Self::RAM_DISP_LINE_WIDTH..(i + 1) * Self::RAM_DISP_LINE_WIDTH]
                )?;
            }
            write!(
                f,
                "{:>2x?}",
                &self.ram[(Self::RAM_DISP_LINES - 1) * Self::RAM_DISP_LINE_WIDTH..]
            )
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
    const RAM_DISP_LINE_WIDTH: usize = 32;
    const RAM_DISP_LINES: usize = Self::RAM_SIZE / Self::RAM_DISP_LINE_WIDTH;

    const FREQ: u32 = 700;

    const RAM_SIZE: usize = 4096;

    const RAM_ROM_START: usize = 0x200;

    const FONT_SIZE: usize = 80;

    const FONT_START: usize = 0x50;
    const FONT: [u8; Self::FONT_SIZE] = [
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

    const DISPLAY_WIDTH: usize = 64;
    const DISPLAY_HEIGHT: usize = 32;
    const DISPLAY_EMPTY: Display = [[false; Self::DISPLAY_WIDTH]; Self::DISPLAY_HEIGHT];

    pub fn new(freq: Option<u32>, font: Option<Font>, options: Option<Chip8VMOptions>) -> Self {
        Chip8VM {
            ram: Chip8VM::init_ram(font.unwrap_or(Self::FONT)),
            display: Self::DISPLAY_EMPTY,
            registers: Registers {
                pc: U12::try_from(Self::RAM_ROM_START).expect("RAM_ROM_START is small enough"),
                ..Registers::default()
            },
            timers: TimersWrapper::new(),
            stack: Vec::new(),
            freq: freq.unwrap_or(Self::FREQ),
            options: options.unwrap_or_default(),
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        assert!(
            rom.len() <= Self::RAM_SIZE - Self::RAM_ROM_START,
            "Rom to big: {}B for {}B available",
            rom.len(),
            Self::RAM_SIZE - Self::RAM_ROM_START
        );
        self.debugln(&format!("Loaded rom of size {}B", rom.len()));
        self.ram[Self::RAM_ROM_START..(Self::RAM_ROM_START + rom.len())]
            .copy_from_slice(&rom[..(Self::RAM_ROM_START + rom.len() - Self::RAM_ROM_START)]);
    }

    pub fn load_rom_from_file(&mut self, rom: &str) {
        self.debugln(&format!("Loading rom from file '{rom}'"));
        let f = std::fs::File::open(rom).expect("file exists");
        let mut reader = std::io::BufReader::new(f);
        let mut rom: Vec<u8> = Vec::new();

        reader.read_to_end(&mut rom).expect("can read file");

        self.load_rom(&rom);
    }
    pub fn pre_run(&mut self) {
        let (lock, cvar) = &*self.timers.lock;
        let mut started = lock.lock().unwrap();
        *started = true;
        // We notify the condvar that the value has changed.
        cvar.notify_one();
    }
    pub fn run_once(&mut self) {
        let instruction = self.fetch_instruction();
        self.debug(&format!("input (raw,decoded): {instruction:x},"));

        let instruction = Chip8Instr::from(instruction);
        self.debugln(&format!("{instruction:?}"));
        self.incr_pc();
        self.execute(instruction);
        self.debugln(&format!("{self:?}"));
    }
    pub fn run(&mut self) {
        self.pre_run();
        loop {
            let time_start = Instant::now();
            let instruction = self.fetch_instruction();
            self.debug(&format!("input (raw,decoded): {instruction:x},"));

            let instruction = Chip8Instr::from(instruction);
            self.debugln(&format!("{instruction:?}"));
            self.incr_pc();
            self.execute(instruction);
            self.debugln(&format!("{self:?}"));
            thread::sleep(Duration::from_secs_f64(
                (1. / self.freq as f64 - time_start.elapsed().as_secs_f64()).max(0.),
            ));
        }
    }

    fn execute(&mut self, instruction: Chip8Instr) {
        match instruction {
            Chip8Instr::Clear => {
                self.display = Self::DISPLAY_EMPTY;
                if !self.options.hide_display {
                    self.display();
                }
            }
            Chip8Instr::Return => {
                self.registers.pc = self.stack.pop().expect("return to be called after a call")
            }
            Chip8Instr::Jump(nnn) => self.registers.pc = nnn,
            Chip8Instr::Call(nnn) => self.stack.push({
                let tmp = self.registers.pc;
                self.registers.pc = nnn;
                tmp
            }),
            Chip8Instr::IfNE(x, nn) => {
                if self.registers.get(x) == nn {
                    self.incr_pc();
                }
            }
            Chip8Instr::IfE(x, nn) => {
                if self.registers.get(x) != nn {
                    self.incr_pc();
                }
            }
            Chip8Instr::IfRNE(x, y) => {
                if self.registers.get(x) == self.registers.get(y) {
                    self.incr_pc();
                }
            }
            Chip8Instr::Set(vx, nn) => self.registers.set(vx, nn),
            Chip8Instr::Add(vx, nn) => {
                self.registers
                    .set(vx, self.registers.get(vx).wrapping_add(nn));
            }
            Chip8Instr::SetR(x, y) => self.registers.set(x, self.registers.get(y)),
            Chip8Instr::BitOp(x, y, op) => {
                let r = match op {
                    1 => self.registers.get(x) | self.registers.get(y),
                    2 => self.registers.get(x) & self.registers.get(y),
                    3 => self.registers.get(x) ^ self.registers.get(y),
                    _ => panic!("Oopsy"),
                };
                self.registers.set(x, r);
            }
            Chip8Instr::ArithmOp(x, y, op) => {
                let (r, mut o) = match op {
                    4 => self.registers.get(x).overflowing_add(self.registers.get(y)),
                    5 => self.registers.get(x).overflowing_sub(self.registers.get(y)),
                    7 => self.registers.get(y).overflowing_sub(self.registers.get(x)),
                    _ => panic!("Oopsy"),
                };
                if op == 5 || op == 7 {
                    o = !o;
                }
                self.registers.set(15, o as u8);
                self.registers.set(x, r);
            }
            Chip8Instr::ShiftOp(x, y, op) => {
                if self.options.old_shift {
                    self.registers.set(x, self.registers.get(y))
                }
                let v = self.registers.get(x);
                let (r, b) = match op {
                    6 => ((v & (0xFE)) >> 1, v & 1 == 1),
                    0xE => ((v & (0x7F)) << 1, v & 128 != 0),
                    _ => panic!("Oopsy"),
                };
                self.registers.set(x, r);
                self.registers.set(15, b as u8);
            }
            Chip8Instr::IfRE(x, y) => {
                if self.registers.get(x) != self.registers.get(y) {
                    self.incr_pc();
                }
            }
            Chip8Instr::SetI(nnn) => self.registers.i = nnn,
            Chip8Instr::JumpOff(nnn) => {
                if self.options.new_jump_off {
                    self.registers.pc = nnn + self.registers.get((nnn >> 8) as u8) as U12;
                } else {
                    self.registers.pc = nnn + self.registers.get(0) as U12;
                }
            }
            Chip8Instr::Rand(x, nn) => {
                let rand: u8 = rand::random();
                self.registers.set(x, nn & rand)
            }
            Chip8Instr::Display(vx, vy, n) => {
                let x = self.registers.get(vx) % (Self::DISPLAY_WIDTH as u8);
                let y = self.registers.get(vy) % (Self::DISPLAY_HEIGHT as u8);
                let sprite_addr = self.registers.i;
                let sprite_height = n;
                self.draw_sprite(x, y, sprite_addr, sprite_height);
                if !self.options.hide_display {
                    self.display();
                }
            }
            Chip8Instr::KeyUp(_x) => {
                println!("KeyUp")
            }
            Chip8Instr::KeyDown(_x) => {
                println!("KeyDown");
            }
            Chip8Instr::GetDelay(x) => {
                println!("Delay");
                self.registers
                    .set(x, self.timers.timers.lock().unwrap().delay);
            }
            Chip8Instr::GetKey(x) => {
                let mut buf = String::new();
                std::io::stdin()
                    .read_line(&mut buf)
                    .expect("Failed to read line");
                let key = buf.chars().next().expect("input");

                let key = match key {
                    k if k.is_ascii_digit() => k as u8 - b'0',
                    k if k.is_ascii_lowercase() => k as u8 - b'a' + 10,
                    k if k.is_ascii_uppercase() => k as u8 - b'A' + 10,
                    _ => 16,
                };
                if key < 16 {
                    self.registers.set(x, key & 0xF);
                }
            }
            Chip8Instr::SetDelay(x) => {
                self.timers.timers.lock().unwrap().delay = self.registers.get(x)
            }
            Chip8Instr::SetBuzzer(x) => {
                self.timers.timers.lock().unwrap().buzzer = self.registers.get(x)
            }
            Chip8Instr::IncrI(x) => self.registers.i += self.registers.get(x) as u16,
            Chip8Instr::Char(x) => self.registers.i = self.char_index(self.registers.get(x)),
            Chip8Instr::Decimal(x) => {
                let x = self.registers.get(x);
                self.ram[self.registers.i as usize] = x / 100;
                self.ram[self.registers.i as usize + 1] = (x % 100) / 10;
                self.ram[self.registers.i as usize + 2] = x % 10;
            }
            Chip8Instr::Save(x) => {
                for i in 0..=x {
                    self.ram[self.registers.i as usize + i as usize] = self.registers.get(i);
                }
                if self.options.incr_i_when_mem {
                    self.registers.i += x as u16;
                }
            }
            Chip8Instr::Load(x) => {
                for i in 0..=x {
                    self.registers
                        .set(i, self.ram[self.registers.i as usize + i as usize]);
                }
                if self.options.incr_i_when_mem {
                    self.registers.i += x as u16;
                }
            }
            _ => panic!("Not implemented ({:?})", instruction),
        }
    }

    fn fetch_instruction(&self) -> u16 {
        let first_byte = self.ram[self.registers.pc as usize];
        let second_byte = self.ram[(self.registers.pc + 1) as usize];
        u16::from_be_bytes([first_byte, second_byte])
    }

    fn display(&self) {
        thread::sleep(Duration::from_secs_f64(1_f64 / 60_f64));
        if !self.options.keep_display {
            print!("{esc}c", esc = 27 as char);
        }
        for y in 0..Self::DISPLAY_HEIGHT {
            for x in 0..Self::DISPLAY_WIDTH {
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
            if curr_y >= Self::DISPLAY_HEIGHT as u8 {
                break;
            }
            for curr_x in x..x + 8 {
                if curr_x >= Self::DISPLAY_WIDTH as u8 {
                    break;
                }
                let pixel_x = x as usize + 8 - (curr_x as usize - x as usize);
                if pixel_x >= Self::DISPLAY_WIDTH {
                    break;
                }
                let pixel = &mut self.display[curr_y as usize][pixel_x];
                let sprite_value = ((sprite_data[(curr_y - y) as usize] >> (curr_x - x)) & 1) == 1;
                if *pixel && sprite_value {
                    self.registers.set(15, 1);
                } else {
                    self.registers.set(15, 0);
                }
                *pixel ^= sprite_value;
            }
        }
    }

    fn char_index(&self, c: u8) -> U12 {
        Self::FONT_START as U12 + 5 * (c & 0xF) as U12
    }
    fn incr_pc(&mut self) {
        self.registers.pc += 2;
    }
    fn init_ram(font: Font) -> Ram {
        let mut ram = [0; Self::RAM_SIZE];
        ram[Self::FONT_START..(Self::FONT_START + Self::FONT_SIZE)]
            .copy_from_slice(&font[..(Self::FONT_START + Self::FONT_SIZE - Self::FONT_START)]);
        ram
    }

    fn debugln(&self, msg: &str) {
        if self.options.debug {
            println!("{msg}");
        }
    }

    fn debug(&self, msg: &str) {
        if self.options.debug {
            print!("{msg}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_new() {
        let vm = Chip8VM::new(None, None, None);
        assert_eq!(
            vm.ram[Chip8VM::FONT_START..Chip8VM::FONT_START + Chip8VM::FONT_SIZE],
            Chip8VM::FONT
        );
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
