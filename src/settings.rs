use directories::ProjectDirs;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Disable all write actions (e.g pull, merge)
    pub read_only: bool,
    /// Disables the alternate screen for printing in the terminal
    pub no_alternate_screen: bool,
}

#[derive(Debug)]
pub enum SettingsError {
    /// Configuration file was not found
    ConfigNotFound,
    /// General IO Error (e.g. permission error)
    Io(std::io::Error),
    /// The config file contains invalid syntax
    InvalidConfig(String),
}

impl ToString for SettingsError {
    fn to_string(&self) -> String {
        match self {
            Self::ConfigNotFound => {
                format!("Config file {} not found", get_config_path().display())
            }
            Self::Io(e) => e.to_string(),
            Self::InvalidConfig(e) => e.to_owned(),
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, SettingsError> {
        let config_path = get_config_path();

        let mut config_file =
            File::open(config_path).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => SettingsError::ConfigNotFound,
                _ => SettingsError::Io(e),
            })?;

        let mut config_string = String::new();
        config_file
            .read_to_string(&mut config_string)
            .map_err(SettingsError::Io)?;

        let settings = toml::from_str(&config_string)
            .map_err(|e| SettingsError::InvalidConfig(e.to_string()))?;

        Ok(settings)
    }
}

fn get_config_path() -> PathBuf {
    let r = ProjectDirs::from("com", "Verco", "Verco");
    let project_dirs = r.expect("no valid home directory path");
    let config_dir = project_dirs.config_dir();

    config_dir.join("verco.toml")
}
