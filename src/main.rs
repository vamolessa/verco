use crossterm::tty::IsTty;

mod custom_actions;
mod git_actions;
mod hg_actions;
mod input;
mod repositories;
mod revision_shortcut;
mod scroll_view;
mod select;
mod tui;
mod tui_util;
mod version_control_actions;
mod worker;

fn main() {
    if !std::io::stdin().is_tty() {
        eprintln!("not tty");
        return;
    }

    ctrlc::set_handler(|| {}).unwrap();
    if let Some(version_control) = repositories::get_current_version_control() {
        let custom_actions =
            custom_actions::CustomAction::load_custom_actions();
        tui::show_tui(version_control, custom_actions);
    } else {
        eprintln!("no repository found");
    }
}
