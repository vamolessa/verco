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
    use worker::Task;
    let mut command = std::process::Command::new("less");
    command.arg("asdsadasd");
    let task1 = worker::ChildTask::from_command(command).unwrap();
    let mut command = std::process::Command::new("echo");
    command.arg("asdsadasd");
    let task2 = worker::ChildTask::from_command(command).unwrap();
    let mut command = std::process::Command::new("echo");
    command.arg("matheus");
    let task3 = worker::ChildTask::from_command(command).unwrap();
    let task1: Box<dyn Task<Output = _>> = Box::new(task1);
    let task2: Box<dyn Task<Output = _>> = Box::new(task2);
    let task3: Box<dyn Task<Output = _>> = Box::new(task3);
    let tasks = vec![task1, task2, task3];
    let mut task = worker::serial(tasks, worker::child_aggragator);
    //let mut task = task1;

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

    if crossterm::tty::IsTty::is_tty(&std::io::stdin()) {
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
