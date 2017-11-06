use std::process::Command;

pub struct VersionControl {}

pub enum Action {
	Status,
}

impl VersionControl {
	pub fn on_action(&self, action: Action) -> Result<String, String> {
		return match action {
			Action::Status => self.status(),
		};
	}

	fn status(&self) -> Result<String, String> {
		let output = Command::new("git")
			.arg("-c color.status=always status")
			.output()
			.expect("Could not run 'status'");

		return Ok(String::from_utf8_lossy(&output.stdout[..]).into_owned());
	}
}
