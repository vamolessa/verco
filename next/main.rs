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

    if std::env::set_current_dir(&root).is_err() {
        eprintln!("could not set current dir to {:?}", root);
        return;
    }

    ctrlc::set_handler(|| {}).unwrap();

    application::run(backend);
}

