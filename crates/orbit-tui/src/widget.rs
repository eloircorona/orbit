use crossterm::event::KeyCode;

pub struct TextInput {
    pub value: String,
    pub cursor: usize,
    #[allow(dead_code)]
    pub label: &'static str,
    pub placeholder: &'static str,
}

impl TextInput {
    pub fn new(label: &'static str, placeholder: &'static str) -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            label,
            placeholder,
        }
    }

    pub fn with_value(mut self, v: &str) -> Self {
        self.value = v.to_string();
        self.cursor = self.value.chars().count();
        self
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Returns `true` if the key was consumed by this input.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char(c) => {
                let byte_pos = self.char_to_byte(self.cursor);
                self.value.insert(byte_pos, c);
                self.cursor += 1;
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    let byte_pos = self.char_to_byte(self.cursor);
                    self.value.remove(byte_pos);
                }
                true
            }
            KeyCode::Delete => {
                let len = self.value.chars().count();
                if self.cursor < len {
                    let byte_pos = self.char_to_byte(self.cursor);
                    self.value.remove(byte_pos);
                }
                true
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor < self.value.chars().count() {
                    self.cursor += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.value.chars().count();
                true
            }
            _ => false,
        }
    }

    /// Render the value with a visible cursor marker inserted at position.
    pub fn display(&self, focused: bool) -> String {
        if !focused {
            if self.value.is_empty() {
                return self.placeholder.to_string();
            }
            return self.value.clone();
        }
        // Insert cursor marker
        let mut chars: Vec<char> = self.value.chars().collect();
        let pos = self.cursor.min(chars.len());
        chars.insert(pos, '│');
        chars.iter().collect()
    }

    fn char_to_byte(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }
}
