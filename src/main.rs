use std::{env, io::Write};

mod application;
mod backend;
mod mode;
mod ui;

fn main() {
    let mut args = env::args();
    args.next();
    if let Some(arg) = args.next() {
        if args.next().is_some() {
            eprintln!("too many args");
        } else {
            match &arg[..] {
                "-h" | "--help" => {
                    let name = env!("CARGO_PKG_NAME");
                    let version = env!("CARGO_PKG_VERSION");
                    println!("{} v{}", name, version);
                    println!();
                    println!("\t-h --help\tprint this help message and exit");
                    println!("\t-v --version\tprint version number and exit");
                }
                "-v" | "--version" => {
                    print!("{}", env!("CARGO_PKG_VERSION"));
                }
                arg => eprintln!("invalid argument '{}'", arg),
            }
        }
        return;
    }

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

    {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        crossterm::execute!(
            &mut stdout,
            crossterm::terminal::SetTitle(root.as_os_str().to_string_lossy()),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::cursor::Hide,
        )
        .unwrap();
        crossterm::terminal::enable_raw_mode().unwrap();
        stdout.flush().unwrap();
    }

    ctrlc::set_handler(|| {}).unwrap();
    application::run(backend);

    {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        crossterm::execute!(
            &mut stdout,
            crossterm::style::ResetColor,
            crossterm::cursor::Show,
            crossterm::terminal::LeaveAlternateScreen,
        )
        .unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();
    }
}

