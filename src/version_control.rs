use std::process::Command;
use std::iter::once;
use std::ops::Add;

use actions::{Action, Actions};

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

	pub fn run_action<FI, FO>(
		&self,
		action: &str,
		on_input: FI,
		on_output: FO,
	) -> Result<(), String>
	where
		FI: Fn() -> String,
		FO: FnMut(Result<&str, &str>),
	{
		let on_in = on_input;
		let mut on_out = on_output;

		match self.actions.iter().find(|&a| a.name == action) {
			Some(act) => {
				for command in &act.commands {
					let result = match command.prompt {
						Some(ref prompt) => {
							on_out(Ok(&prompt[..]));
							let input = on_in();
							let exec = simple_format(&command.exec[..], &input[..]);
							run(self.current_dir, &exec[..])
						}
						None => run(self.current_dir, &command.exec[..]),
					};

					match result {
						Ok(output) => on_out(Ok(&output[..])),
						Err(error) => on_out(Err(&error[..])),
					}
				}

				return Ok(());
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

	return match Command::new(exec)
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
	};
}

fn simple_format(format: &str, arg: &str) -> String {
	format
		.splitn(2, '$')
		.take(1)
		.chain(once(arg))
		.chain(format.splitn(2, '$').skip(1))
		.fold(String::from(""), |xs, x| xs.add(x))
}
