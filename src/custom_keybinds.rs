// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use iced::keyboard::{Key, Modifiers};
use std::collections::HashMap;

pub struct CustomKeybinds<T> {
    keybinds: HashMap<Keybind, T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Keybind {
    key: Key,
    modifiers: Modifiers,
}

impl<T: Clone> CustomKeybinds<T> {
    pub fn new() -> Self {
        Self {
            keybinds: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_keybind(&mut self, key: Key, modifiers: Modifiers, value: T) {
        self.keybinds.insert(Keybind { key, modifiers }, value);
    }

    #[allow(dead_code)]
    pub fn remove_keybind(&mut self, key: Key, modifiers: Modifiers) {
        self.keybinds.remove(&Keybind { key, modifiers });
    }

    #[allow(dead_code)]
    pub fn has_keybind(&self, key: Key, modifiers: Modifiers) -> bool {
        self.keybinds.contains_key(&Keybind { key, modifiers })
    }

    #[allow(dead_code)]
    pub fn get_keybind(&self, key: Key, modifiers: Modifiers) -> Option<&T> {
        self.keybinds.get(&Keybind { key, modifiers })
    }

    #[allow(dead_code)]
    pub fn get_keybind_copy(&self, key: Key, modifiers: Modifiers) -> Option<T> {
        self.keybinds.get(&Keybind { key, modifiers }).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::key::Named;

    #[test]
    fn custom_keybinds_can_be_added_and_removed() {
        let mut keybinds = CustomKeybinds::new();

        keybinds.add_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty(), "Left");

        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowRight), Modifiers::empty()),
            false
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            Some(&"Left")
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowRight), Modifiers::empty()),
            None
        );

        keybinds.add_keybind(Key::Named(Named::ArrowRight), Modifiers::empty(), "Right");

        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowRight), Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            Some(&"Left")
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowRight), Modifiers::empty()),
            Some(&"Right")
        );

        keybinds.remove_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty());

        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            false
        );
        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowRight), Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            None
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowRight), Modifiers::empty()),
            Some(&"Right")
        );
    }

    #[test]
    fn custom_keybinds_can_have_different_modifiers() {
        let mut keybinds = CustomKeybinds::new();

        keybinds.add_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty(), "Left");

        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowLeft), Modifiers::SHIFT),
            false
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            Some(&"Left")
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowLeft), Modifiers::SHIFT),
            None
        );

        keybinds.add_keybind(Key::Named(Named::ArrowLeft), Modifiers::SHIFT, "Right");

        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            true
        );
        assert_eq!(
            keybinds.has_keybind(Key::Named(Named::ArrowLeft), Modifiers::SHIFT),
            true
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowLeft), Modifiers::empty()),
            Some(&"Left")
        );
        assert_eq!(
            keybinds.get_keybind(Key::Named(Named::ArrowLeft), Modifiers::SHIFT),
            Some(&"Right")
        );
    }
}
