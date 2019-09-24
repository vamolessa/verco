mod git_actions;
mod hg_actions;
mod repositories;
mod revision_shortcut;
mod select;
mod tui;
mod version_control_actions;

fn main() {
	ctrlc::set_handler(move || {}).unwrap();

	if let Some(version_control) = repositories::get_current_version_control() {
		tui::show_tui(vec![version_control]);
	} else {
		eprintln!("no repository found");
	}
}
//