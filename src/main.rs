extern crate termion;

mod tui;
use tui::Tui;

mod version_control;
use version_control::VersionControl;

fn main() {
	let _guard = termion::init();

	let vc = VersionControl {};
	let mut tui = Tui::init();
	tui.show(&vc);
}
