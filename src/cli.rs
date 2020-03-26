use dirs;
use std::{fs, path::PathBuf, process::Command};
use structopt::StructOpt;

use crate::version_control_actions::handle_command;

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

pub fn handle_cli_options() -> Result<bool, &'static str> {
    let opt = Opt::from_args();
    match opt.command {
        None => return Ok(true),
        Some(OptCommand::Git(command)) => match command {
            VersionControlCommand::GenerateSshKey { filename } => {
                let mut dir = dirs::home_dir().ok_or("could not open home directory")?;
                dir.push(".ssh");

                let mut file = dir.clone();
                file.push(&filename);
                if file.exists() {
                    return Err("ssh key already exists");
                }

                fs::create_dir_all(&dir).map_err(|_| "could not open .ssh directory")?;
                generate_ssh_key(&dir, &filename);
                file.set_extension("pub");

                let pub_key =
                    fs::read_to_string(&file).map_err(|_| "could not open ssh key public file")?;
                println!("{}", pub_key);
            }
        },
    }

    Ok(false)
}

#[cfg(target_os = "windows")]
fn generate_ssh_key(dir: &PathBuf, filename: &str) {
    let mut c = Command::new("git");
    c.current_dir(dir)
        .arg("bash")
        .arg("--hide")
        .arg("-c")
        .arg(format!("ssh-keygen -q -t rsa -f ./{} -N ''", filename));
    match handle_command(&mut c) {
        Ok(_) => (),
        Err(error) => eprintln!("{}", error),
    }
}

#[cfg(not(target_os = "windows"))]
fn generate_ssh_key(dir: &PathBuf, filename: &str) {
    let mut c = Command::new("ssh-keygen");
    c.current_dir(dir)
        .arg("-q")
        .arg("-t")
        .arg("-rsa")
        .arg("-f")
        .arg(format!("./{}", filename))
        .arg("-N")
        .arg("");
    match handle_command(&mut c) {
        Ok(_) => (),
        Err(error) => eprintln!("{}", error),
    }
}
