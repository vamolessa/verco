use crossterm::tty::IsTty;

mod custom_commands;
mod git_actions;
mod hg_actions;
mod input;
mod repositories;
mod revision_shortcut;
mod scroll_view;
mod select;
mod settings;
mod tui;
mod tui_util;
mod version_control_actions;

fn main() {
    if !std::io::stdin().is_tty() {
        eprintln!("not tty");
        return;
    }

    ctrlc::set_handler(|| {}).unwrap();
    if let Some(version_control) = repositories::get_current_version_control() {
        let custom_commands =
            custom_commands::CustomCommand::load_custom_commands();
        tui::show_tui(version_control, custom_commands);
    } else {
        eprintln!("no repository found");
    }
}
