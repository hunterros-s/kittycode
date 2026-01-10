use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct InputHistory {
    entries: VecDeque<String>,
    max_size: usize,
    cursor: Option<usize>,
    draft: String,
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::new(100)
    }
}

impl InputHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_size,
            cursor: None,
            draft: String::new(),
        }
    }

    pub fn push(&mut self, entry: String) {
        if entry.is_empty() {
            return;
        }

        // Avoid duplicate consecutive entries
        if self.entries.front() == Some(&entry) {
            return;
        }

        self.entries.push_front(entry);

        if self.entries.len() > self.max_size {
            self.entries.pop_back();
        }

        self.reset();
    }

    pub fn navigate_up(&mut self, current_text: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }

        match self.cursor {
            None => {
                self.draft = current_text.to_string();
                self.cursor = Some(0);
                self.entries.get(0).map(|s| s.as_str())
            }
            Some(idx) => {
                let next_idx = idx + 1;
                if next_idx < self.entries.len() {
                    self.cursor = Some(next_idx);
                    self.entries.get(next_idx).map(|s| s.as_str())
                } else {
                    None
                }
            }
        }
    }

    pub fn navigate_down(&mut self) -> Option<&str> {
        match self.cursor {
            None => None,
            Some(0) => {
                self.cursor = None;
                Some(self.draft.as_str())
            }
            Some(idx) => {
                let next_idx = idx - 1;
                self.cursor = Some(next_idx);
                self.entries.get(next_idx).map(|s| s.as_str())
            }
        }
    }

    pub fn reset(&mut self) {
        self.cursor = None;
        self.draft.clear();
    }

    pub fn is_browsing(&self) -> bool {
        self.cursor.is_some()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_history_is_empty() {
        let h = InputHistory::new(10);
        assert!(h.is_empty());
        assert!(!h.is_browsing());
    }

    #[test]
    fn push_and_navigate() {
        let mut h = InputHistory::new(10);
        h.push("first".to_string());
        h.push("second".to_string());

        assert_eq!(h.len(), 2);

        let up1 = h.navigate_up("current");
        assert_eq!(up1, Some("second"));
        assert!(h.is_browsing());

        let up2 = h.navigate_up("current");
        assert_eq!(up2, Some("first"));

        let up3 = h.navigate_up("current");
        assert_eq!(up3, None); // at end

        let down1 = h.navigate_down();
        assert_eq!(down1, Some("second"));

        let down2 = h.navigate_down();
        assert_eq!(down2, Some("current")); // draft restored

        assert!(!h.is_browsing());
    }

    #[test]
    fn navigate_up_empty_history() {
        let mut h = InputHistory::new(10);
        assert_eq!(h.navigate_up("text"), None);
    }

    #[test]
    fn navigate_down_not_browsing() {
        let mut h = InputHistory::new(10);
        h.push("entry".to_string());
        assert_eq!(h.navigate_down(), None);
    }

    #[test]
    fn reset_clears_browsing() {
        let mut h = InputHistory::new(10);
        h.push("entry".to_string());
        h.navigate_up("draft");
        assert!(h.is_browsing());

        h.reset();
        assert!(!h.is_browsing());
    }

    #[test]
    fn push_respects_max_size() {
        let mut h = InputHistory::new(3);
        h.push("1".to_string());
        h.push("2".to_string());
        h.push("3".to_string());
        h.push("4".to_string());

        assert_eq!(h.len(), 3);

        // Oldest entry "1" should be gone
        h.navigate_up("");
        assert_eq!(h.navigate_up(""), Some("3"));
        h.navigate_up("");
        assert_eq!(h.navigate_up(""), None); // "1" is gone
    }

    #[test]
    fn push_empty_string_ignored() {
        let mut h = InputHistory::new(10);
        h.push("".to_string());
        assert!(h.is_empty());
    }

    #[test]
    fn push_duplicate_consecutive_ignored() {
        let mut h = InputHistory::new(10);
        h.push("same".to_string());
        h.push("same".to_string());
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn push_duplicate_non_consecutive_allowed() {
        let mut h = InputHistory::new(10);
        h.push("a".to_string());
        h.push("b".to_string());
        h.push("a".to_string());
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn draft_preserved_during_browsing() {
        let mut h = InputHistory::new(10);
        h.push("old".to_string());

        h.navigate_up("my draft text");
        assert_eq!(h.navigate_down(), Some("my draft text"));
    }
}
