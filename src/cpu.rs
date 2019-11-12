use std::convert::From;
use std::io::Read;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::sleep;
use std::time::{Duration, Instant};

use log::*;
use rand::prelude::*;

use crate::{
	timer::{DelayTimer, SoundTimer},
	Display, Key, Ram,
};

#[derive(Debug)]
pub struct Cpu {
	v: [u8; 16],
	i: u16,
	stack: [u16; 16],
	st: SoundTimer,
	dt: DelayTimer,
	pub pc: u16,
	sp: u16,
	key: Option<Key>,
}

impl Cpu {
	pub fn new() -> Cpu {
		let mut dt = DelayTimer::new();
		dt.start();
		let mut st = SoundTimer::new();
		st.start();
		Cpu {
			v: [0; 16],
			i: 0x200,
			stack: [0; 16],
			st,
			dt,
			pc: 0x200,
			sp: 0,
			key: None,
		}
	}

	fn draw(&self, dsp: &mut Box<Display>, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()> {
		dsp.draw(x, y, data)
	}

	fn clear(&self, dsp: &mut Box<Display>) -> Result<(), ()> {
		dsp.clear();
		Ok(())
	}

	pub fn run(&mut self, ram: &mut Ram, dsp: &mut Box<Display>, inp: &mut mpsc::Receiver<Key>) {
		loop {
			if self.pc >= 0xFFF || (self.pc + 1) >= 0xFFF {
				break;
			}

			self.tick(ram, dsp, inp);
		}
	}

	pub fn tick(&mut self, ram: &mut Ram, io: &mut Box<Display>, inp: &mut mpsc::Receiver<Key>) {
		use Res::*;
		let pc = self.pc as usize;
		let o1: u8 = ram.buf[pc] >> 4;
		let o2: u8 = ram.buf[pc] & 0xf;
		let o3: u8 = ram.buf[pc + 1] >> 4;
		let o4: u8 = ram.buf[pc + 1] & 0xf;
		let res = match (o1, o2, o3, o4) {
			(0x0, 0x0, 0xE, 0x0) => {
				trace!("00E0 - CLS");
				self.clear(io).unwrap();
				Next
			}
			(0x0, 0x0, 0xE, 0xE) => {
				trace!("00EE - RET");
				let pc = self.stack[self.sp as usize - 1];
				self.sp -= 1;
				Jump(pc + 2)
			}
			(0x0, n1, n2, n3) => {
				let nnn = addr(n1, n2, n3);
				trace!("0nnn - SYS {}", nnn);
				Jump(nnn)
			}
			(0x1, n1, n2, n3) => {
				let nnn = addr(n1, n2, n3);
				trace!("1nnn - JP {}", nnn);
				Jump(nnn)
			}
			(0x2, n1, n2, n3) => {
				let nnn = addr(n1, n2, n3);
				trace!("2nnn - CALL {}", nnn);
				self.stack[self.sp as usize] = self.pc;
				self.sp += 1;
				Jump(nnn)
			}
			(0x3, x, k1, k2) => {
				let kk = var(k1, k2);
				let vx = self.v[idx(x)];
				trace!("SE V{}({}) K({})", x, vx, kk);
				if vx == kk {
					Skip
				} else {
					Next
				}
			}
			(0x4, x, k1, k2) => {
				let kk = var(k1, k2);
				trace!("SNE Vx({}) K({})", x, kk);
				if self.v[idx(x)] != kk {
					Skip
				} else {
					Next
				}
			}
			(0x5, x, y, 0x0) => {
				trace!("SE Vx({}), Vy({})", x, y);
				if self.v[idx(x)] == self.v[idx(y)] {
					Skip
				} else {
					Next
				}
			}
			(0x6, x, k1, k2) => {
				let kk = var(k1, k2);
				trace!("6xkk - LD V{}={}", x, kk);
				self.v[idx(x)] = kk;
				Next
			}
			(0x7, x, k1, k2) => {
				let x = idx(x);
				let kk = var(k1, k2);
				trace!("7xkk - ADD V{} {}", x, kk);
				self.v[x] = self.v[x].overflowing_add(kk).0;
				Next
			}
			(0x8, x, y, 0x0) => {
				trace!("8xy0 - LD V{} V{}", x, y);
				self.v[idx(x)] = self.v[idx(y)];
				Next
			}
			(0x8, x, y, 0x1) => {
				trace!("8xy1 - OR V{} V{}", x, y);
				self.v[idx(x)] |= self.v[idx(y)];
				Next
			}
			(0x8, x, y, 0x2) => {
				trace!("8xy2 - AND V{} V{}", x, y);
				self.v[idx(x)] &= self.v[idx(y)];
				Next
			}
			(0x8, x, y, 0x3) => {
				trace!("8xy3 - XOR V{} V{}", x, y);
				self.v[idx(x)] ^= self.v[idx(y)];
				Next
			}
			(0x8, x, y, 0x4) => {
				trace!("8xy4 - ADD V{} V{}", x, y);
				let xy = self.v[idx(x)] as u16 + self.v[idx(y)] as u16;
				if xy > 0xff {
					self.v[0xf] = 1;
				} else {
					self.v[0xf] = 0;
				}
				self.v[idx(x)] = (xy & 0xff) as u8;
				Next
			}
			(0x8, x, y, 0x5) => {
				let vx = self.v[idx(x)];
				let vy = self.v[idx(y)];
				trace!("8xy5 - SUB V{}={} V{}={}", x, vx, y, vy);
				let (val, overflow) = vx.overflowing_sub(vy);
				if !overflow {
					self.v[0xf] = 1;
				} else {
					self.v[0xf] = 0;
				}
				self.v[idx(x)] = val;
				Next
			}
			(0x8, x, y, 0x6) => {
				trace!("8xy6 - SHR V{} V{}", x, y);
				self.v[0xf] = self.v[idx(x)] & 0x1;
				self.v[idx(x)] /= 2;
				Next
			}
			(0x8, x, y, 0x7) => {
				let vx = self.v[idx(x)];
				let vy = self.v[idx(y)];
				trace!("8xy7 - SUBN V{}={} V{}={}", x, vx, y, vy);
				let (val, overflow) = vy.overflowing_sub(vx);

				if !overflow {
					self.v[0xf] = 1;
				} else {
					self.v[0xf] = 0;
				}
				self.v[idx(x)] = val;
				Next
			}
			(0x8, x, y, 0xE) => {
				trace!("8xyE - SHL V{} V{}", x, y);
				self.v[0xf] = self.v[idx(x)] >> 7;
				self.v[idx(x)] = self.v[idx(x)].overflowing_mul(2).0;
				Next
			}
			(0x9, x, y, 0x0) => {
				trace!("SNE V{}, V{}", x, y);
				if self.v[idx(x)] != self.v[idx(y)] {
					Skip
				} else {
					Next
				}
			}
			(0xA, n1, n2, n3) => {
				self.i = addr(n1, n2, n3);
				trace!("Annn - LD I, {}", self.i);
				Next
			}
			(0xB, n1, n2, n3) => {
				let i = addr(n1, n2, n3) + self.v[0] as u16;
				trace!("Bnnn - JP V0, {:x}", i);
				Jump(i)
			}
			(0xC, x, k1, k2) => {
				let rnd: u8 = random();
				let kk = var(k1, k2);
				trace!("Cxkk - RND V{} {}", x, kk);
				self.v[idx(x)] = rnd & kk;
				Next
			}
			(0xD, x, y, n) => {
				let vx = self.v[idx(x)];
				let vy = self.v[idx(y)];
				let since = self.i as usize;
				let until = since + idx(n);
				let bytes = (&ram.buf[since..until]).to_vec();
				trace!(
					"Dxyn - DRW V{}={}, V{}={}, nibble={}, bytes={:?}",
					x,
					vx,
					y,
					vy,
					n,
					bytes
				);
				self.v[0xf] = self.draw(io, vx, vy, bytes).unwrap();
				Next
			}
			(0xE, x, 0x9, 0xE) => {
				trace!("Ex9E - SKP V{}={}", x, self.v[idx(x)]);
				if let Some(key) = self.key(inp) {
					if key.0 == self.v[idx(x)] {
						self.key = None;
						Skip
					} else {
						Next
					}
				} else {
					Next
				}
			}
			(0xE, x, 0xA, 0x1) => {
				trace!("ExA1 - SKNP V{}={}", x, self.v[idx(x)]);
				if let Some(key) = self.key(inp) {
					if key.0 == self.v[idx(x)] {
						self.key = None;
						Next
					} else {
						Skip
					}
				} else {
					Skip
				}
			}
			(0xF, x, 0x0, 0x7) => {
				trace!("Fx07 - LD Vx, DT");
				self.v[idx(x)] = self.dt.get();
				Next
			}
			(0xF, x, 0x0, 0xA) => {
				trace!("Fx0A - LD Vx, K");
				let mut pressed = false;
				if let Some(c) = self.key(inp) {
					debug!("Got {:?}", c);
					self.v[idx(x)] = c.0;
					pressed = true;
				}

				if pressed {
					Next
				} else {
					Jump(self.pc)
				}
			}
			(0xF, x, 0x1, 0x5) => {
				trace!("Fx15 - LD DT, Vx");
				self.dt.set(self.v[idx(x)]);
				Next
			}
			(0xF, x, 0x1, 0x8) => {
				trace!("Fx18 - LD ST, Vx");
				self.st.set(self.v[idx(x)]);
				Next
			}
			(0xF, x, 0x1, 0xE) => {
				trace!("ADD I, Vx");
				self.i += self.v[idx(x)] as u16;
				Next
			}
			(0xF, x, 0x2, 0x9) => {
				let vx = self.v[idx(x)];
				trace!("Fx29 - LD F, Vx={}", vx);
				self.i = fontaddr(vx);
				Next
			}
			(0xF, x, 0x3, 0x3) => {
				trace!("Fx33 - LD B, Vx");
				let i = self.i as usize;
				let vx = self.v[idx(x)];
				ram.buf[i] = (vx / 100) as u8 % 10;
				ram.buf[i + 1] = (vx / 10) as u8 % 10;
				ram.buf[i + 2] = vx % 10;
				Next
			}
			(0xF, x, 0x5, 0x5) => {
				trace!("Fx55 - LD [I], V{}", x);
				for n in 0..x + 1 {
					ram.buf[self.i as usize + idx(n)] = self.v[idx(n)];
				}
				Next
			}
			(0xF, x, 0x6, 0x5) => {
				trace!("Fx65 - LD V{}, I={}", x, self.i);
				for n in 0..x + 1 {
					self.v[idx(n)] = ram.buf[self.i as usize + idx(n)];
				}
				Next
			}
			_ => {
				panic!("N/A {:x}{:x}{:x}{:x}", o1, o2, o3, o4);
				Next
			}
		};

		// Determine the next `pc`.
		match res {
			Next => {
				self.pc += 2;
			}
			Skip => {
				self.pc += 4;
			}
			Jump(loc) => {
				self.pc = loc;
			}
		}
		self.dump();
	}

	fn key(&mut self, inp: &mut mpsc::Receiver<Key>) -> Option<Key> {
		inp.try_recv().ok().or(self.key).map(|k| {
			debug!("receiving key {:?}", k);
			self.key = Some(k);
			k
		})
	}

	pub fn dump(&self) {
		trace!(
			" v{:?} i={}({:x}) stack={:?} sp={} pc={}({:x}) dt={}",
			self.v,
			self.i,
			self.i,
			self.stack,
			self.sp,
			self.pc,
			self.pc,
			self.dt
		);
	}
}

pub enum Res {
	/// Increase `pc` by 2.
	Next,
	/// Increase `pc` by 4.
	Skip,
	/// Set `pc` the value.
	Jump(u16),
}

fn addr(n1: u8, n2: u8, n3: u8) -> u16 {
	((n1 as u16) << 8) + ((n2 as u16) << 4) + n3 as u16
}

fn fontaddr(n: u8) -> u16 {
	n as u16 * 5
}

fn var(x1: u8, x2: u8) -> u8 {
	((x1 as u8) << 4) + x2 as u8
}

fn idx(x: u8) -> usize {
	x as usize
}
