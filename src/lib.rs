mod cpu;
mod ram;
mod timer;

pub use cpu::Cpu;
pub use ram::Ram;

use std::sync::mpsc;

pub struct Chip8 {
	cpu: Cpu,
	pub ram: Ram,
	pub dsp: Box<Display>,
	pub inp: mpsc::Receiver<Key>,
}

impl Chip8 {
	pub fn new(dsp: Box<Display>, inp: mpsc::Receiver<Key>) -> Self {
		let cpu = Cpu::new();
		let ram = Ram::new();
		Chip8 { cpu, ram, dsp, inp }
	}

	pub fn run(&mut self) {
		self.cpu.run(&mut self.ram, &mut self.dsp, &mut self.inp);
	}

	pub fn tick(&mut self) {
		self.cpu.tick(&mut self.ram, &mut self.dsp, &mut self.inp);
	}
}

pub trait Display {
	fn draw(&mut self, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()>;
	fn clear(&self);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key(pub u8);

impl std::convert::From<char> for Key {
	fn from(c: char) -> Key {
		match c {
			'1' => Key(0x1),
			'2' => Key(0x2),
			'3' => Key(0x3),
			'q' => Key(0x4),
			'w' => Key(0x5),
			'e' => Key(0x6),
			'a' => Key(0x7),
			's' => Key(0x8),
			'd' => Key(0x9),
			'z' => Key(0xA),
			'x' => Key(0x0),
			'c' => Key(0xB),
			'4' => Key(0xC),
			'r' => Key(0xD),
			'f' => Key(0xE),
			'v' => Key(0xF),
			_ => Key(0x99),
		}
	}
}
