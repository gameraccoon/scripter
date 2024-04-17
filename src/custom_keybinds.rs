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

        let keybindA = Keybind {
            key: keyboard::KeyCode::Key1,
            modifiers: keyboard::Modifiers::default(),
        };
        let keybindB = Keybind {
            key: keyboard::KeyCode::Key2,
            modifiers: keyboard::Modifiers::default(),
        };

        keybinds.add_keybind(keybindA, "A");

        assert_eq!(keybinds.has_keybind(keybindA), true);
        assert_eq!(keybinds.has_keybind(keybindB), false);
        assert_eq!(keybinds.get_keybind(keybindA), Some(&"A"));
        assert_eq!(keybinds.get_keybind(keybindB), None);

        keybinds.add_keybind(keybindB, "B");

        assert_eq!(keybinds.has_keybind(keybindA), true);
        assert_eq!(keybinds.has_keybind(keybindB), true);
        assert_eq!(keybinds.get_keybind(keybindA), Some(&"A"));
        assert_eq!(keybinds.get_keybind(keybindB), Some(&"B"));

        keybinds.remove_keybind(keybindA);

        assert_eq!(keybinds.has_keybind(keybindA), false);
        assert_eq!(keybinds.has_keybind(keybindB), true);
        assert_eq!(keybinds.get_keybind(keybindA), None);
        assert_eq!(keybinds.get_keybind(keybindB), Some(&"B"));
    }
}
