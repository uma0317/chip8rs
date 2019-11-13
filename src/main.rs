use chip8rs::{Chip8, Display};
use env_logger;
use log::*;
use std::{
	env,
	io::{stdin, stdout, Write},
	sync::{mpsc, Arc, Mutex},
	thread::sleep,
	time::{Duration, Instant},
};
use termion::event::Key;
use termion::{input::TermRead, raw::IntoRawMode};

const WIDTH: usize = 64;

const HEIGHT: usize = 32;

struct DisplayAdaptor {
	console: Arc<Mutex<Console>>,
}

impl DisplayAdaptor {
	fn new(console: Arc<Mutex<Console>>) -> Self {
		DisplayAdaptor { console }
	}
}

impl Display for DisplayAdaptor {
	fn draw(&mut self, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()> {
		self.console.lock().unwrap().draw(x, y, data)
	}

	fn clear(&self) {
		self.console.lock().unwrap().clear();
	}
}

enum Filler {
	Fill,
	Unfill,
}

struct Console {
	keyboard: mpsc::Sender<chip8rs::Key>,
	curr: [[u8; HEIGHT]; WIDTH],
}

impl Console {
	fn new(keyboard: mpsc::Sender<chip8rs::Key>) -> Self {
		let console = Console {
			keyboard,
			curr: [[0; HEIGHT]; WIDTH],
		};
		console.clear();
		console
	}

	fn draw(&mut self, x: u8, y: u8, data: Vec<u8>) -> Result<u8, ()> {
		let x = x as usize;
		let y = y as usize;
		let mut vf = 0;
		for (iy, b) in data.iter().enumerate() {
			let next = bitarray(*b);
			for (ix, nb) in next.iter().enumerate() {
				if x + ix >= WIDTH || y + iy >= HEIGHT {
					continue;
				}

				let cb = self.curr[x + ix][y + iy];
				match (cb, nb) {
					(0, 0) => {}
					(0, 1) | (1, 0) => {
						self.draw_pixel(x + ix, y + iy, Filler::Fill);
					}
					(1, 1) => {
						vf = 1;
						self.draw_pixel(x + ix, y + iy, Filler::Unfill);
					}
					_ => {
						panic!("Illegal bit value: cb={}, nb={}", cb, nb);
					}
				}
			}
		}

		Ok(vf)
	}

	fn draw_pixel(&mut self, x: usize, y: usize, fill: Filler) {
		let mut stdout = stdout().into_raw_mode().unwrap();

		match fill {
			Filler::Fill => self.curr[x][y] = 1,
			Filler::Unfill => self.curr[x][y] = 0,
		}
		match fill {
			Filler::Fill => write!(
				stdout,
				"{}{}",
				termion::cursor::Goto(x as u16 + 1, y as u16 + 1),
				"■"
			)
			.unwrap(),
			Filler::Unfill => write!(
				stdout,
				"{}{}",
				termion::cursor::Goto(x as u16 + 1, y as u16 + 1),
				" "
			)
			.unwrap(),
		}
		stdout.flush().unwrap();
	}

	fn flush(&mut self) {
		let mut stdout = stdout().into_raw_mode().unwrap();
		write!(
			stdout,
			"{}{}",
			termion::cursor::Hide,
			termion::cursor::Goto(1, 1),
		)
		.unwrap();
		stdout.flush().unwrap();

		for y in 0..HEIGHT {
			for x in 0..WIDTH {
				if self.curr[x][y] == 0 {
					write!(stdout, " ").unwrap();
				} else {
					write!(stdout, "■").unwrap();
				}
			}
			write!(stdout, "\n\r").unwrap();
		}
		stdout.flush().unwrap();
	}

	fn clear(&self) {
		let mut stdout = stdout().into_raw_mode().unwrap();
		write!(
			stdout,
			"{}{}",
			termion::cursor::Goto(1, 1),
			termion::clear::All
		)
		.unwrap();
	}
}
fn bitarray(byte: u8) -> Vec<u8> {
	let mut s = Vec::new();
	for n in 0..8 {
		s.push((byte >> (7 - n)) & 0x1);
	}
	s
}

fn emuloop(mut chip8: Chip8, console: Arc<Mutex<Console>>, args: Vec<String>) -> Result<(), ()> {
	println!("start loop");
	println!("{:?}", args);

	let frame = Duration::from_millis((1000 / args[2].parse::<u64>().unwrap()) as u64);
	let console2 = console.clone();
	std::thread::spawn(move || loop {
		let mut stdout = stdout().into_raw_mode().unwrap();
		stdout.flush().unwrap();
		let stdin = stdin();
		for c in stdin.keys() {
			match c.unwrap() {
				Key::Esc => std::process::exit(0),
				Key::Char(c) => match console2.lock() {
					Ok(con) => {
						let k = chip8rs::Key::from(c);
						if k.0 != 0x99 {
							debug!("sending key {:?}", c);
							con.keyboard
								.send(k)
								.map_err(|e| error!("Keyboard error: {}", e))
								.unwrap();
						}
					}
					Err(e) => error!("{}", e),
				},
				_ => {}
			}
		}
	});
	match console.lock() {
		Ok(mut c) => {
			c.flush();
		}
		Err(e) => {
			error!("Unable to unlock Console: {}", e);
		}
	}
	loop {
		let now = Instant::now();
		chip8.tick();
		if let Some(remaining) = frame.checked_sub(now.elapsed()) {
			sleep(remaining)
		}
	}
}
fn run(args: Vec<String>) -> Result<(), ()> {
	let (itx, irx) = mpsc::channel::<chip8rs::Key>();
	let console = Arc::new(Mutex::new(Console::new(itx)));
	let adaptor = DisplayAdaptor::new(console.clone());

	let mut chip8 = Chip8::new(Box::new(adaptor), irx);
	let file = std::fs::File::open(&args[1]).unwrap();

	match chip8.ram.load(file) {
		Ok(_) => {}
		Err(e) => println!("{}", e),
	}

	emuloop(chip8, console, args)
}

fn main() -> Result<(), ()> {
	env::set_var("RUST_LOG", "info");
	env_logger::init();
	let args: Vec<String> = env::args().collect();
	run(args)
}
