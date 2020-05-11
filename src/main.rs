use crossterm::tty::IsTty;
use worker::Task;

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
    let mut command = std::process::Command::new("less");
    command.arg("asdsadasd");
    let task1 = worker::ChildTask::from_command(command).unwrap();
    let mut command = std::process::Command::new("echo");
    command.arg("asdsadasd");
    let task2 = worker::ChildTask::from_command(command).unwrap();
    let mut task = task1.and_also(Box::new(task2), worker::child_aggragator);

    loop {
        match task.poll() {
            std::task::Poll::Ready(Ok(output)) => {
                println!("output: {}", output);
                break;
            }
            std::task::Poll::Ready(Err(error)) => {
                println!("error: {}", error);
                break;
            }
            std::task::Poll::Pending => std::thread::sleep_ms(10),
        }
    }
    return;

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
