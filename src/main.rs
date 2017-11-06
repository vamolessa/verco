extern crate termion;

mod tui;

mod version_control;
use version_control::VersionControl;

fn main() {
	let _guard = termion::init();

	let version_control = VersionControl {};
	tui::show(&version_control);
}
