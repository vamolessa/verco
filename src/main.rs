use std::{env, io};

mod application;
mod backend;
mod mode;
mod platform;
mod ui;

/*
fn main2() {
    fn block_sigwinch() {
        unsafe {
            let mut signals = std::mem::zeroed();
            let result = libc::sigemptyset(&mut signals);
            if result == -1 {
                panic!("could not create signal fd");
            }
            let result = libc::sigaddset(&mut signals, libc::SIGWINCH);
            if result == -1 {
                panic!("could not create signal fd");
            }
            let result = libc::sigprocmask(
                libc::SIG_BLOCK,
                &signals,
                std::ptr::null_mut(),
            );
            if result == -1 {
                panic!("could not create signal fd");
            }
        }
    }

    block_sigwinch();
    let resize_signal_fd = unsafe {
        let mut signals = std::mem::zeroed();
        let result = libc::sigprocmask(0, std::ptr::null_mut(), &mut signals);
        let fd = libc::signalfd(-1, &signals, 0);
        if fd == -1 {
            panic!("could not create signal fd");
        }
        fd
    };

    let _ = std::thread::spawn(move || {
        //block_sigwinch();

        loop {
            println!("{:?}", platform::Platform::terminal_size());
            let mut buf = [0; std::mem::size_of::<libc::signalfd_siginfo>()];
            if platform::PlatformEventReader::read(resize_signal_fd, &mut buf)
                != Ok(buf.len())
            {
                panic!("could not read from signal fd");
            }
        }
    })
    .join();
}
*/

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

    let (platform, platform_event_reader) = match platform::Platform::new() {
        Some(platform) => platform,
        None => return,
    };

    {
        use io::Write;
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        stdout.write_all(ui::BEGIN_TITLE_CODE).unwrap();
        stdout
            .write_all(root.as_os_str().to_string_lossy().as_bytes())
            .unwrap();
        stdout.write_all(ui::END_TITLE_CODE).unwrap();
        stdout.write_all(ui::ENTER_ALTERNATE_BUFFER_CODE).unwrap();
        stdout.write_all(ui::HIDE_CURSOR_CODE).unwrap();
        stdout.flush().unwrap();
    }

    application::run(platform_event_reader, backend);

    {
        use io::Write;
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        stdout.write_all(ui::RESET_STYLE_CODE).unwrap();
        stdout.write_all(ui::SHOW_CURSOR_CODE).unwrap();
        stdout.write_all(ui::EXIT_ALTERNATE_BUFFER_CODE).unwrap();
        stdout.flush().unwrap();
    }

    drop(platform);
}

