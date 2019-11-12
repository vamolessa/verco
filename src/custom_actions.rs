pub struct CustomAction {
    pub key_chord: String,
    pub command: String,
}

impl CustomAction {
    pub fn load_custom_actions(base_path: &str) -> Vec<CustomAction> {
        Vec::new()
    }
}
