#[derive(Default)]
pub struct RevisionShortcut {
    hashes: Vec<String>,
}

impl RevisionShortcut {
    pub fn max() -> usize {
        'Z' as usize - 'A' as usize
    }

    pub fn update_hashes(&mut self, hashes: Vec<String>) {
        self.hashes = hashes;
    }

    pub fn replace_occurrences(&self, text: &mut String) {
        let mut final_text = String::new();
        final_text.push_str(&text[..]);

        for (i, h) in self.hashes.iter().take(Self::max()).enumerate() {
            if let Some(shortcut) = std::char::from_u32('A' as u32 + i as u32) {
                let replacement = format!("{} ({})", h, shortcut);
                final_text = final_text.replacen(h, &replacement[..], 1);
            }
        }

        text.clear();
        text.push_str(&final_text[..]);
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
