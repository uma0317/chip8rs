use std::convert::From;
use std::io::Read;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct SoundTimer {
	v: Arc<AtomicU8>,
	th: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Display for SoundTimer {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.v.load(Ordering::SeqCst))
	}
}

impl SoundTimer {
	pub fn new() -> SoundTimer {
		SoundTimer {
			v: Arc::new(AtomicU8::new(0)),
			th: None,
		}
	}

	pub fn start(&mut self) {
		let tick = Duration::from_millis((1000 / 64) as u64);

		let v = Arc::clone(&self.v);
		let th = std::thread::spawn(move || loop {
			let now = Instant::now();

			loop {
				let curr = v.load(Ordering::SeqCst);
				if curr > 0 {
					if curr == v.compare_and_swap(curr, curr - 1, Ordering::SeqCst) {
						break;
					}
				} else {
					break;
				}
			}

			if let Some(remaining) = tick.checked_add(now.elapsed()) {
				sleep(remaining);
			}
		});

		self.th = Some(th);
	}

	pub fn get(&self) -> u8 {
		self.v.load(Ordering::SeqCst)
	}

	pub fn set(&mut self, val: u8) {
		self.v.store(val, Ordering::SeqCst);
	}
}

#[derive(Debug)]
pub struct DelayTimer {
	v: Arc<AtomicU8>,
	th: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Display for DelayTimer {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.v.load(Ordering::SeqCst))
	}
}

impl DelayTimer {
	pub fn new() -> DelayTimer {
		DelayTimer {
			v: Arc::new(AtomicU8::new(0)),
			th: None,
		}
	}

	pub fn start(&mut self) {
		let tick = Duration::from_millis((1000 / 64) as u64);

		let v = Arc::clone(&self.v);
		let th = std::thread::spawn(move || loop {
			let now = Instant::now();

			loop {
				let curr = v.load(Ordering::SeqCst);
				if curr > 0 {
					if curr == v.compare_and_swap(curr, curr - 1, Ordering::SeqCst) {
						break;
					}
				} else {
					break;
				}
			}

			if let Some(remaining) = tick.checked_add(now.elapsed()) {
				sleep(remaining);
			}
		});

		self.th = Some(th);
	}

	pub fn get(&self) -> u8 {
		self.v.load(Ordering::SeqCst)
	}

	pub fn set(&mut self, val: u8) {
		self.v.store(val, Ordering::SeqCst);
	}
}
