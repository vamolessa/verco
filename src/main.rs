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
    let task1 = worker::CommandTask::Waiting(command);
    let mut command = std::process::Command::new("echo");
    command.arg("asdsadasd");
    let task2 = worker::CommandTask::Waiting(command);
    let mut command = std::process::Command::new("echo");
    command.arg("matheus");
    let task3 = worker::CommandTask::Waiting(command);

    let mut tasks = worker::task_vec();
    tasks.push(Box::new(task1));
    tasks.push(Box::new(task2));
    tasks.push(Box::new(task3));

    let aggregated = worker::parallel(tasks, worker::child_aggragator);

    let wk = worker::Worker::new();
    wk.send_task(aggregated);
    match wk.receive_result() {
        Ok(output) => println!("ok: {}", output),
        Err(error) => println!("error: {}", error),
    }
    //let task_count = tasks.len();
    //for task in tasks.drain(..) {
    //    wk.send_task(task);
    //}
    //for _ in 0..task_count {
    //    match wk.receive_result() {
    //        Ok(output) => println!("ok: {}", output),
    //        Err(error) => println!("error: {}", error),
    //    }
    //}

    //loop {
    //    match task.poll() {
    //        std::task::Poll::Ready(Ok(output)) => {
    //            println!("output: {}", output);
    //            break;
    //        }
    //        std::task::Poll::Ready(Err(error)) => {
    //            println!("error: {}", error);
    //            break;
    //        }
    //        std::task::Poll::Pending => std::thread::sleep_ms(10),
    //    }
    //}
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
