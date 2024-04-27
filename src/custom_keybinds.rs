// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use iced::keyboard::{KeyCode, Modifiers};
use std::collections::HashMap;

pub struct CustomKeybinds<T> {
    keybinds: HashMap<Keybind, T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Keybind {
    key: KeyCode,
    modifiers: Modifiers,
}

impl<T: Clone> CustomKeybinds<T> {
    pub fn new() -> Self {
        Self {
            keybinds: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_keybind(&mut self, key: KeyCode, modifiers: Modifiers, value: T) {
        self.keybinds.insert(Keybind { key, modifiers }, value);
    }

    #[allow(dead_code)]
    pub fn remove_keybind(&mut self, key: KeyCode, modifiers: Modifiers) {
        self.keybinds.remove(&Keybind { key, modifiers });
    }

    #[allow(dead_code)]
    pub fn has_keybind(&self, key: KeyCode, modifiers: Modifiers) -> bool {
        self.keybinds.contains_key(&Keybind { key, modifiers })
    }

    #[allow(dead_code)]
    pub fn get_keybind(&self, key: KeyCode, modifiers: Modifiers) -> Option<&T> {
        self.keybinds.get(&Keybind { key, modifiers })
    }

    #[allow(dead_code)]
    pub fn get_keybind_copy(&self, key: KeyCode, modifiers: Modifiers) -> Option<T> {
        self.keybinds.get(&Keybind { key, modifiers }).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_keybinds_can_be_added_and_removed() {
        let mut keybinds = CustomKeybinds::new();

        keybinds.add_keybind(KeyCode::Key1, Modifiers::empty(), "A");

        assert_eq!(
            keybinds.has_keybind(KeyCode::Key1, Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.has_keybind(KeyCode::Key2, Modifiers::empty()),
            false
        );
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key1, Modifiers::empty()),
            Some(&"A")
        );
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key2, Modifiers::empty()),
            None
        );

        keybinds.add_keybind(KeyCode::Key2, Modifiers::empty(), "B");

        assert_eq!(
            keybinds.has_keybind(KeyCode::Key1, Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.has_keybind(KeyCode::Key2, Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key1, Modifiers::empty()),
            Some(&"A")
        );
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key2, Modifiers::empty()),
            Some(&"B")
        );

        keybinds.remove_keybind(KeyCode::Key1, Modifiers::empty());

        assert_eq!(
            keybinds.has_keybind(KeyCode::Key1, Modifiers::empty()),
            false
        );
        assert_eq!(
            keybinds.has_keybind(KeyCode::Key2, Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key1, Modifiers::empty()),
            None
        );
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key2, Modifiers::empty()),
            Some(&"B")
        );
    }

    #[test]
    fn custom_keybinds_can_have_different_modifiers() {
        let mut keybinds = CustomKeybinds::new();

        keybinds.add_keybind(KeyCode::Key1, Modifiers::empty(), "A");

        assert_eq!(
            keybinds.has_keybind(KeyCode::Key1, Modifiers::empty()),
            true
        );
        assert_eq!(keybinds.has_keybind(KeyCode::Key1, Modifiers::SHIFT), false);
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key1, Modifiers::empty()),
            Some(&"A")
        );
        assert_eq!(keybinds.get_keybind(KeyCode::Key1, Modifiers::SHIFT), None);

        keybinds.add_keybind(KeyCode::Key1, Modifiers::SHIFT, "B");

        assert_eq!(
            keybinds.has_keybind(KeyCode::Key1, Modifiers::empty()),
            true
        );
        assert_eq!(keybinds.has_keybind(KeyCode::Key1, Modifiers::SHIFT), true);
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key1, Modifiers::empty()),
            Some(&"A")
        );
        assert_eq!(
            keybinds.get_keybind(KeyCode::Key1, Modifiers::SHIFT),
            Some(&"B")
        );
    }
}
