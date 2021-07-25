mod application;
mod backend;
mod controller;
mod ui;

fn main() {
    if !crossterm::tty::IsTty::is_tty(&std::io::stdin()) {
        eprintln!("not tty");
        return;
    }

    let (root, backend) = match backend::backend_from_current_repository() {
        Some((root, backend)) => (root, backend),
        None => {
            eprintln!("no repository found");
            return;
        }
    };

    ctrlc::set_handler(|| {}).unwrap();

    application::run(root, backend);
}

