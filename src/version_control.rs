use std::process::Command;
use std::iter::once;
use std::ops::Add;

use actions::{Action, Actions};

pub trait VersionControlIO {
	fn on_in(&mut self) -> Option<String>;
	fn on_out(&mut self, Result<Option<&str>, &str>);
}

pub struct VersionControl<'a> {
	pub actions: &'a Vec<Action>,
	current_dir: &'a str,
}

impl<'a> VersionControl<'a> {
	pub fn find_current<'b>(
		current_dir: &'b str,
		actions: &'b Actions,
	) -> Result<VersionControl<'b>, ()> {
		return match actions.sets.get("git") {
			Some(actions) => Ok(VersionControl {
				current_dir: current_dir,
				actions: actions,
			}),
			None => Err(()),
		};
	}

	pub fn run_action<T: VersionControlIO>(&self, action: &str, io: &mut T) -> Result<(), String> {
		match self.actions.iter().find(|&a| a.name == action) {
			Some(act) => {
				for command in &act.commands {
					let result = match command.prompt {
						Some(ref prompt) => {
							io.on_out(Ok(Some(&prompt[..])));
							match io.on_in() {
								Some(input) => {
									let exec = simple_format(&command.exec[..], &input[..]);
									run(self.current_dir, &exec[..]).map(|o| Some(o))
								}
								None => Ok(None),
							}
						}
						None => run(self.current_dir, &command.exec[..]).map(|o| Some(o)),
					};

					match result {
						Ok(result) => match result {
							Some(output) => io.on_out(Ok(Some(&output[..]))),
							None => io.on_out(Ok(None)),
						},
						Err(error) => io.on_out(Err(&error[..])),
					}
				}

				Ok(())
			}
			None => Err(format!("Could not find action '{}'.", action)),
		}
	}
}

fn run(current_dir: &str, command: &str) -> Result<String, String> {
	let mut command_iter = command.split(" ");
	let exec = command_iter
		.next()
		.ok_or(format!("Invalid command '{}'", command))?;

	match Command::new(exec)
		.current_dir(current_dir)
		.args(command_iter.collect::<Vec<&str>>())
		.output()
	{
		Ok(output) => if output.status.success() {
			Ok(String::from_utf8_lossy(&output.stdout[..]).into_owned())
		} else {
			Err(String::from_utf8_lossy(&output.stderr[..]).into_owned())
		},
		Err(error) => Err(error.to_string()),
	}
}

fn simple_format(format: &str, arg: &str) -> String {
	format
		.splitn(2, '$')
		.take(1)
		.chain(once(arg))
		.chain(format.splitn(2, '$').skip(1))
		.fold(String::from(""), |xs, x| xs.add(x))
}
