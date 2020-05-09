#[derive(Default)]
pub struct RevisionShortcut {
    hashes: Vec<String>,
}

impl RevisionShortcut {
    pub fn max() -> usize {
        'Z' as usize - 'A' as usize + 1
    }

    pub fn update_hashes(&mut self, hashes: Vec<String>) {
        self.hashes = hashes;
    }

    pub fn replace_occurrences(&self, text: &mut String) -> Vec<String> {
        let mut final_text = String::new();
        final_text.push_str(&text[..]);
        let mut hashes = Vec::with_capacity(26);

        // This pattern is weird enough to never appear anywhere in the log.
        let pattern = r"(__VERCO_NODE__([0-9a-fA-F]+)__VERCO_NODE__)";
        let re = regex::Regex::new(pattern).unwrap();

        for (i, capture) in re.captures_iter(&text).enumerate() {
            let occurrence = capture.get(1).unwrap().as_str();
            let hash = capture.get(2).unwrap().as_str();

            if i < Self::max() {
                hashes.push(hash.to_owned());
                let shortcut =
                    std::char::from_u32('A' as u32 + i as u32).unwrap();
                let replacement = format!("{} ({})", hash, shortcut);
                final_text = final_text.replacen(occurrence, &replacement, 1);
            } else {
                final_text = final_text.replacen(occurrence, hash, 1);
            }
        }

        text.clear();
        text.push_str(&final_text[..]);

        hashes
    }

    pub fn get_hash(&self, target: &str) -> Option<&str> {
        if target.len() != 1 {
            return None;
        }

        let shortcut = target.chars().next().unwrap() as i32;
        let index = shortcut - 'A' as i32;
        if index < 0 || index >= self.hashes.len() as i32 {
            return None;
        }

        Some(&self.hashes[index as usize][..])
    }
}
