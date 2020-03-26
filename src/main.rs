mod cli;
mod ctrlc_handler;
mod custom_commands;
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

fn main() {
    match cli::handle_cli_options() {
        Ok(true) => {
            let handler = ctrlc_handler::CtrlcHandler::new();
            let handler_clone = handler.clone();
            ctrlc::set_handler(move || {
                handler_clone.set(true);
            })
            .unwrap();

            if let Some(version_control) = repositories::get_current_version_control() {
                let custom_commands = custom_commands::CustomCommand::load_custom_commands();
                tui::show_tui(version_control, custom_commands, handler);
            } else {
                eprintln!("no repository found");
            }
        }
        Ok(false) => (),
        Err(error) => eprintln!("{}", error),
    }
}
