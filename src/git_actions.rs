use std::process::Command;

pub struct GitActions<'a> {
	pub current_dir: &'a str,
}

impl<'a> GitActions<'a> {
	pub fn status(&self) -> Result<String, String> {
		match Command::new("git")
			.current_dir(self.current_dir)
			.args(&["status"])
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
}
