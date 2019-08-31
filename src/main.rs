mod git_actions;
mod hg_actions;
mod repositories;
mod revision_shortcut;
mod select;
mod tui;
mod version_control_actions;

fn main() {
	ctrlc::set_handler(move || {}).unwrap();

	let version_controls = repositories::get_version_controls().unwrap();
	repositories::set_version_controls(&version_controls).unwrap();

	if version_controls.len() == 0 {
		println!("no repository found");
	} else {
		tui::show_tui(version_controls);
	}
}
