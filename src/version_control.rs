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
		Ok(String::from("aaaeeeeee"))
	}
}
