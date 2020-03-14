mod custom_commands;
mod git_actions;
mod hg_actions;
mod input;
mod repositories;
mod revision_shortcut;
mod select;
mod tui;
mod version_control_actions;

fn main() {
    ctrlc::set_handler(move || {}).unwrap();

    if let Some(version_control) = repositories::get_current_version_control() {
        let custom_commands = custom_commands::CustomCommand::load_custom_commands();
        tui::show_tui(vec![version_control], custom_commands);
    } else {
        eprintln!("no repository found");
    }
}
