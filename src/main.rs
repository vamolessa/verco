extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use std::io::{Write};

mod console;
use console::Console;

fn main() {
	let _guard = termion::init();

	let mut console = Console::new();

	console.clear();

	write!(
		console.stdout,
		"{}{}q to exit. Type stuff, use alt, and so on.{}",
		termion::clear::All,
		termion::cursor::Goto(1, 1),
		termion::cursor::Hide
	).unwrap();

	console.stdout.flush().unwrap();

	for c in console.stdin.keys() {
		write!(
			console.stdout,
			"{}{}",
			termion::cursor::Goto(1, 1),
			termion::clear::CurrentLine
		).unwrap();

		match c.unwrap() {
			Key::Char('q') => break,
			Key::Char(c) => println!("{}", c),
			Key::Alt(c) => println!("^{}", c),
			Key::Ctrl(c) => println!("*{}", c),
			Key::Esc => println!("ESC"),
			Key::Left => println!("←"),
			Key::Right => println!("→"),
			Key::Up => println!("↑"),
			Key::Down => println!("↓"),
			Key::Backspace => println!("×"),
			_ => {}
		}

		console.stdout.flush().unwrap();
	}

	write!(console.stdout, "{}", termion::cursor::Show).unwrap();
}
