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

use std::env;
use std::process::Command;
use structopt::StructOpt;

use version_control_actions::handle_command;

#[derive(StructOpt)]
#[structopt(name = "verco")]
struct Opt {
    #[structopt(subcommand)]
    command: Option<OptCommand>,
}

#[derive(StructOpt)]
enum OptCommand {
    Git(VersionControlCommand),
}

#[derive(StructOpt)]
enum VersionControlCommand {
    GenerateSshKey {
        #[structopt(default_value = "id_rsa")]
        filename: String,
    },
}

fn main() {
    let opt = Opt::from_args();
    let current_dir = env::current_dir().unwrap();
    match opt.command {
        Some(OptCommand::Git(command)) => match command {
            VersionControlCommand::GenerateSshKey { filename } => {
                let mut c = Command::new("git");
                c.current_dir(&current_dir)
                        .arg("bash")
                        .arg("--hide")
                        .arg("--cd-to-home")
                        .arg("-c")
                        .arg(format!("mkdir .ssh 2>/dev/null; ssh-keygen -q -t rsa -f .ssh/{} -N '' 2>/dev/null <<< y >/dev/null", filename));
                match handle_command(&mut c) {
                    Ok(out) => println!("{}", out),
                    Err(error) => eprintln!("{}", error),
                }
            }
        },
        None => {
            let handler = ctrlc_handler::CtrlcHandler::new();
            let handler_clone = handler.clone();
            ctrlc::set_handler(move || {
                handler_clone.set(true);
            })
            .unwrap();

            if let Some(version_control) = repositories::get_current_version_control(&current_dir) {
                let custom_commands = custom_commands::CustomCommand::load_custom_commands();
                tui::show_tui(version_control, custom_commands, handler);
            } else {
                eprintln!("no repository found");
            }
        }
    }
}
