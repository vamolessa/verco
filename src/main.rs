mod application;
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
    if !crossterm::tty::IsTty::is_tty(&std::io::stdin()) {
        eprintln!("not tty");
        return;
    }

    ctrlc::set_handler(|| {}).unwrap();
    if let Some(version_control) = repositories::get_current_version_control() {
        let application = application::Application::new(
            version_control,
            custom_actions::CustomAction::load_custom_actions(),
        );
        tui::show_tui(application);
    } else {
        eprintln!("no repository found");
    }
}
