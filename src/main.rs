extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use std::io::Write;

mod tui;
use tui::Tui;

fn main() {
	let _guard = termion::init();

	let mut tui = Tui::new();

	tui.clear();

	write!(
		tui.stdout,
		"{}{}q to exit. Type stuff, use alt, and so on.{}",
		termion::clear::All,
		termion::cursor::Goto(1, 1),
		termion::cursor::Hide
	).unwrap();

	tui.stdout.flush().unwrap();

	for c in tui.stdin.keys() {
		write!(
			tui.stdout,
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

		tui.stdout.flush().unwrap();
	}

	write!(tui.stdout, "{}", termion::cursor::Show).unwrap();
}
