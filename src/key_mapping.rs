use bitflags::bitflags;
use iced::keyboard::{KeyCode, Modifiers};
use serde::{Deserialize, Deserializer, Serialize};

// this is pretty stupid code, but it solves a few problems:
// 1. we can serialize the keybinds
// 2. we can make sure if the keybinds enum changes the app won't
//    compile, so we can maake an updater for the old condifg
// 3. configs don't need to know about iced
// 4. we can give keybinds user friendly names

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CustomKeyCode {
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    Snapshot,
    /// Scroll Lock.
    Scroll,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    Backspace,
    /// The Enter key.
    Enter,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    NavigateForward,  // also called "Next"
    NavigateBackward, // also called "Prior"
    NextTrack,
    NoConvert,
    OEM102,
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}

bitflags! {
    /// The current state of the keyboard modifiers.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CustomModifiers: u32{
        /// The "shift" key.
        const SHIFT = 0b100;
        // const LSHIFT = 0b010 << 0;
        // const RSHIFT = 0b001 << 0;
        //
        /// The "control" key.
        const CTRL = 0b100 << 3;
        // const LCTRL = 0b010 << 3;
        // const RCTRL = 0b001 << 3;
        //
        /// The "alt" key.
        const ALT = 0b100 << 6;
        // const LALT = 0b010 << 6;
        // const RALT = 0b001 << 6;
        //
        /// The "windows" key on Windows, "command" key on Mac, and
        /// "super" key on Linux.
        const LOGO = 0b100 << 9;
        // const LLOGO = 0b010 << 9;
        // const RLOGO = 0b001 << 9;
    }
}

impl<'de> Serialize for CustomModifiers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut modifiers = String::new();
        // try to save config platform independent, in a way
        // that Command is Ctrl on windows and Ctrl is Cmd on mac

        #[cfg(target_os = "macos")]
        {
            if self.logo() {
                modifiers.push_str("Cmd");
            }
            if self.control() {
                if modifiers.len() == 0 {
                    modifiers.push_str("Ctrl");
                } else {
                    modifiers.push_str("+Ctrl");
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            if self.control() {
                modifiers.push_str("Cmd");
            }
            if self.logo() {
                if modifiers.len() == 0 {
                    modifiers.push_str("Win");
                } else {
                    modifiers.push_str("+Win");
                }
            }
        }

        if self.alt() {
            if modifiers.len() == 0 {
                modifiers.push_str("Alt");
            } else {
                modifiers.push_str("+Alt");
            }
        }
        if self.shift() {
            if modifiers.len() == 0 {
                modifiers.push_str("Shift");
            } else {
                modifiers.push_str("+Shift");
            }
        }
        serializer.serialize_str(modifiers.as_str())
    }
}

impl<'de> Deserialize<'de> for CustomModifiers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut modifiers = CustomModifiers::default();
        String::deserialize(deserializer)?
            .split('+')
            .for_each(|modifier| {
                #[cfg(target_os = "macos")]
                match modifier {
                    "Shift" => modifiers |= CustomModifiers::SHIFT,
                    "Ctrl" => modifiers |= CustomModifiers::CTRL,
                    "Alt" => modifiers |= CustomModifiers::ALT,
                    "Logo" => modifiers |= CustomModifiers::LOGO,
                    "Cmd" => modifiers |= CustomModifiers::LOGO,
                    _ => {}
                }

                #[cfg(not(target_os = "macos"))]
                match modifier {
                    "Shift" => modifiers |= CustomModifiers::SHIFT,
                    "Ctrl" => modifiers |= CustomModifiers::CTRL,
                    "Alt" => modifiers |= CustomModifiers::ALT,
                    "Logo" => modifiers |= CustomModifiers::LOGO,
                    "Cmd" => modifiers |= CustomModifiers::CTRL,
                    // just in case someone inputs that manually
                    "Win" => modifiers |= CustomModifiers::LOGO,
                    _ => {}
                }
            });
        return Ok(modifiers);
    }
}

impl CustomModifiers {
    /// The "command" key.
    ///
    /// This is normally the main modifier to be used for hotkeys.
    ///
    /// On macOS, this is equivalent to `Self::LOGO`.
    /// Ohterwise, this is equivalent to `Self::CTRL`.
    pub const COMMAND: Self = if cfg!(target_os = "macos") {
        Self::LOGO
    } else {
        Self::CTRL
    };

    /// Returns true if the [`SHIFT`] key is pressed in the [`Modifiers`].
    ///
    /// [`SHIFT`]: Self::SHIFT
    pub fn shift(&self) -> bool {
        self.contains(Self::SHIFT)
    }

    /// Returns true if the [`CTRL`] key is pressed in the [`Modifiers`].
    ///
    /// [`CTRL`]: Self::CTRL
    pub fn control(&self) -> bool {
        self.contains(Self::CTRL)
    }

    /// Returns true if the [`ALT`] key is pressed in the [`Modifiers`].
    ///
    /// [`ALT`]: Self::ALT
    pub fn alt(&self) -> bool {
        self.contains(Self::ALT)
    }

    /// Returns true if the [`LOGO`] key is pressed in the [`Modifiers`].
    ///
    /// [`LOGO`]: Self::LOGO
    pub fn logo(&self) -> bool {
        self.contains(Self::LOGO)
    }

    /// Returns true if a "command key" is pressed in the [`Modifiers`].
    ///
    /// The "command key" is the main modifier key used to issue commands in the
    /// current platform. Specifically:
    ///
    /// - It is the `logo` or command key (⌘) on macOS
    /// - It is the `control` key on other platforms
    #[allow(dead_code)]
    pub fn command(&self) -> bool {
        #[cfg(target_os = "macos")]
        let is_pressed = self.logo();

        #[cfg(not(target_os = "macos"))]
        let is_pressed = self.control();

        is_pressed
    }
}

pub fn get_iced_key_code_from_custom_key_code(key: CustomKeyCode) -> KeyCode {
    match key {
        CustomKeyCode::Key1 => KeyCode::Key1,
        CustomKeyCode::Key2 => KeyCode::Key2,
        CustomKeyCode::Key3 => KeyCode::Key3,
        CustomKeyCode::Key4 => KeyCode::Key4,
        CustomKeyCode::Key5 => KeyCode::Key5,
        CustomKeyCode::Key6 => KeyCode::Key6,
        CustomKeyCode::Key7 => KeyCode::Key7,
        CustomKeyCode::Key8 => KeyCode::Key8,
        CustomKeyCode::Key9 => KeyCode::Key9,
        CustomKeyCode::Key0 => KeyCode::Key0,
        CustomKeyCode::A => KeyCode::A,
        CustomKeyCode::B => KeyCode::B,
        CustomKeyCode::C => KeyCode::C,
        CustomKeyCode::D => KeyCode::D,
        CustomKeyCode::E => KeyCode::E,
        CustomKeyCode::F => KeyCode::F,
        CustomKeyCode::G => KeyCode::G,
        CustomKeyCode::H => KeyCode::H,
        CustomKeyCode::I => KeyCode::I,
        CustomKeyCode::J => KeyCode::J,
        CustomKeyCode::K => KeyCode::K,
        CustomKeyCode::L => KeyCode::L,
        CustomKeyCode::M => KeyCode::M,
        CustomKeyCode::N => KeyCode::N,
        CustomKeyCode::O => KeyCode::O,
        CustomKeyCode::P => KeyCode::P,
        CustomKeyCode::Q => KeyCode::Q,
        CustomKeyCode::R => KeyCode::R,
        CustomKeyCode::S => KeyCode::S,
        CustomKeyCode::T => KeyCode::T,
        CustomKeyCode::U => KeyCode::U,
        CustomKeyCode::V => KeyCode::V,
        CustomKeyCode::W => KeyCode::W,
        CustomKeyCode::X => KeyCode::X,
        CustomKeyCode::Y => KeyCode::Y,
        CustomKeyCode::Z => KeyCode::Z,
        CustomKeyCode::Escape => KeyCode::Escape,
        CustomKeyCode::F1 => KeyCode::F1,
        CustomKeyCode::F2 => KeyCode::F2,
        CustomKeyCode::F3 => KeyCode::F3,
        CustomKeyCode::F4 => KeyCode::F4,
        CustomKeyCode::F5 => KeyCode::F5,
        CustomKeyCode::F6 => KeyCode::F6,
        CustomKeyCode::F7 => KeyCode::F7,
        CustomKeyCode::F8 => KeyCode::F8,
        CustomKeyCode::F9 => KeyCode::F9,
        CustomKeyCode::F10 => KeyCode::F10,
        CustomKeyCode::F11 => KeyCode::F11,
        CustomKeyCode::F12 => KeyCode::F12,
        CustomKeyCode::F13 => KeyCode::F13,
        CustomKeyCode::F14 => KeyCode::F14,
        CustomKeyCode::F15 => KeyCode::F15,
        CustomKeyCode::F16 => KeyCode::F16,
        CustomKeyCode::F17 => KeyCode::F17,
        CustomKeyCode::F18 => KeyCode::F18,
        CustomKeyCode::F19 => KeyCode::F19,
        CustomKeyCode::F20 => KeyCode::F20,
        CustomKeyCode::F21 => KeyCode::F21,
        CustomKeyCode::F22 => KeyCode::F22,
        CustomKeyCode::F23 => KeyCode::F23,
        CustomKeyCode::F24 => KeyCode::F24,
        CustomKeyCode::Snapshot => KeyCode::Snapshot,
        CustomKeyCode::Scroll => KeyCode::Scroll,
        CustomKeyCode::Pause => KeyCode::Pause,
        CustomKeyCode::Insert => KeyCode::Insert,
        CustomKeyCode::Home => KeyCode::Home,
        CustomKeyCode::Delete => KeyCode::Delete,
        CustomKeyCode::End => KeyCode::End,
        CustomKeyCode::PageDown => KeyCode::PageDown,
        CustomKeyCode::PageUp => KeyCode::PageUp,
        CustomKeyCode::Left => KeyCode::Left,
        CustomKeyCode::Up => KeyCode::Up,
        CustomKeyCode::Right => KeyCode::Right,
        CustomKeyCode::Down => KeyCode::Down,
        CustomKeyCode::Backspace => KeyCode::Backspace,
        CustomKeyCode::Enter => KeyCode::Enter,
        CustomKeyCode::Space => KeyCode::Space,
        CustomKeyCode::Compose => KeyCode::Compose,
        CustomKeyCode::Caret => KeyCode::Caret,
        CustomKeyCode::Numlock => KeyCode::Numlock,
        CustomKeyCode::Numpad0 => KeyCode::Numpad0,
        CustomKeyCode::Numpad1 => KeyCode::Numpad1,
        CustomKeyCode::Numpad2 => KeyCode::Numpad2,
        CustomKeyCode::Numpad3 => KeyCode::Numpad3,
        CustomKeyCode::Numpad4 => KeyCode::Numpad4,
        CustomKeyCode::Numpad5 => KeyCode::Numpad5,
        CustomKeyCode::Numpad6 => KeyCode::Numpad6,
        CustomKeyCode::Numpad7 => KeyCode::Numpad7,
        CustomKeyCode::Numpad8 => KeyCode::Numpad8,
        CustomKeyCode::Numpad9 => KeyCode::Numpad9,
        CustomKeyCode::NumpadAdd => KeyCode::NumpadAdd,
        CustomKeyCode::NumpadDivide => KeyCode::NumpadDivide,
        CustomKeyCode::NumpadDecimal => KeyCode::NumpadDecimal,
        CustomKeyCode::NumpadComma => KeyCode::NumpadComma,
        CustomKeyCode::NumpadEnter => KeyCode::NumpadEnter,
        CustomKeyCode::NumpadEquals => KeyCode::NumpadEquals,
        CustomKeyCode::NumpadMultiply => KeyCode::NumpadMultiply,
        CustomKeyCode::NumpadSubtract => KeyCode::NumpadSubtract,
        CustomKeyCode::AbntC1 => KeyCode::AbntC1,
        CustomKeyCode::AbntC2 => KeyCode::AbntC2,
        CustomKeyCode::Apostrophe => KeyCode::Apostrophe,
        CustomKeyCode::Apps => KeyCode::Apps,
        CustomKeyCode::Asterisk => KeyCode::Asterisk,
        CustomKeyCode::At => KeyCode::At,
        CustomKeyCode::Ax => KeyCode::Ax,
        CustomKeyCode::Backslash => KeyCode::Backslash,
        CustomKeyCode::Calculator => KeyCode::Calculator,
        CustomKeyCode::Capital => KeyCode::Capital,
        CustomKeyCode::Colon => KeyCode::Colon,
        CustomKeyCode::Comma => KeyCode::Comma,
        CustomKeyCode::Convert => KeyCode::Convert,
        CustomKeyCode::Equals => KeyCode::Equals,
        CustomKeyCode::Grave => KeyCode::Grave,
        CustomKeyCode::Kana => KeyCode::Kana,
        CustomKeyCode::Kanji => KeyCode::Kanji,
        CustomKeyCode::LAlt => KeyCode::LAlt,
        CustomKeyCode::LBracket => KeyCode::LBracket,
        CustomKeyCode::LControl => KeyCode::LControl,
        CustomKeyCode::LShift => KeyCode::LShift,
        CustomKeyCode::LWin => KeyCode::LWin,
        CustomKeyCode::Mail => KeyCode::Mail,
        CustomKeyCode::MediaSelect => KeyCode::MediaSelect,
        CustomKeyCode::MediaStop => KeyCode::MediaStop,
        CustomKeyCode::Minus => KeyCode::Minus,
        CustomKeyCode::Mute => KeyCode::Mute,
        CustomKeyCode::MyComputer => KeyCode::MyComputer,
        CustomKeyCode::NavigateForward => KeyCode::NavigateForward,
        CustomKeyCode::NavigateBackward => KeyCode::NavigateBackward,
        CustomKeyCode::NextTrack => KeyCode::NextTrack,
        CustomKeyCode::NoConvert => KeyCode::NoConvert,
        CustomKeyCode::OEM102 => KeyCode::OEM102,
        CustomKeyCode::Period => KeyCode::Period,
        CustomKeyCode::PlayPause => KeyCode::PlayPause,
        CustomKeyCode::Plus => KeyCode::Plus,
        CustomKeyCode::Power => KeyCode::Power,
        CustomKeyCode::PrevTrack => KeyCode::PrevTrack,
        CustomKeyCode::RAlt => KeyCode::RAlt,
        CustomKeyCode::RBracket => KeyCode::RBracket,
        CustomKeyCode::RControl => KeyCode::RControl,
        CustomKeyCode::RShift => KeyCode::RShift,
        CustomKeyCode::RWin => KeyCode::RWin,
        CustomKeyCode::Semicolon => KeyCode::Semicolon,
        CustomKeyCode::Slash => KeyCode::Slash,
        CustomKeyCode::Sleep => KeyCode::Sleep,
        CustomKeyCode::Stop => KeyCode::Stop,
        CustomKeyCode::Sysrq => KeyCode::Sysrq,
        CustomKeyCode::Tab => KeyCode::Tab,
        CustomKeyCode::Underline => KeyCode::Underline,
        CustomKeyCode::Unlabeled => KeyCode::Unlabeled,
        CustomKeyCode::VolumeDown => KeyCode::VolumeDown,
        CustomKeyCode::VolumeUp => KeyCode::VolumeUp,
        CustomKeyCode::Wake => KeyCode::Wake,
        CustomKeyCode::WebBack => KeyCode::WebBack,
        CustomKeyCode::WebFavorites => KeyCode::WebFavorites,
        CustomKeyCode::WebForward => KeyCode::WebForward,
        CustomKeyCode::WebHome => KeyCode::WebHome,
        CustomKeyCode::WebRefresh => KeyCode::WebRefresh,
        CustomKeyCode::WebSearch => KeyCode::WebSearch,
        CustomKeyCode::WebStop => KeyCode::WebStop,
        CustomKeyCode::Yen => KeyCode::Yen,
        CustomKeyCode::Copy => KeyCode::Copy,
        CustomKeyCode::Paste => KeyCode::Paste,
        CustomKeyCode::Cut => KeyCode::Cut,
    }
}

#[allow(dead_code)]
pub fn get_custom_key_code_from_iced_key_code(key: KeyCode) -> CustomKeyCode {
    match key {
        KeyCode::Key1 => CustomKeyCode::Key1,
        KeyCode::Key2 => CustomKeyCode::Key2,
        KeyCode::Key3 => CustomKeyCode::Key3,
        KeyCode::Key4 => CustomKeyCode::Key4,
        KeyCode::Key5 => CustomKeyCode::Key5,
        KeyCode::Key6 => CustomKeyCode::Key6,
        KeyCode::Key7 => CustomKeyCode::Key7,
        KeyCode::Key8 => CustomKeyCode::Key8,
        KeyCode::Key9 => CustomKeyCode::Key9,
        KeyCode::Key0 => CustomKeyCode::Key0,
        KeyCode::A => CustomKeyCode::A,
        KeyCode::B => CustomKeyCode::B,
        KeyCode::C => CustomKeyCode::C,
        KeyCode::D => CustomKeyCode::D,
        KeyCode::E => CustomKeyCode::E,
        KeyCode::F => CustomKeyCode::F,
        KeyCode::G => CustomKeyCode::G,
        KeyCode::H => CustomKeyCode::H,
        KeyCode::I => CustomKeyCode::I,
        KeyCode::J => CustomKeyCode::J,
        KeyCode::K => CustomKeyCode::K,
        KeyCode::L => CustomKeyCode::L,
        KeyCode::M => CustomKeyCode::M,
        KeyCode::N => CustomKeyCode::N,
        KeyCode::O => CustomKeyCode::O,
        KeyCode::P => CustomKeyCode::P,
        KeyCode::Q => CustomKeyCode::Q,
        KeyCode::R => CustomKeyCode::R,
        KeyCode::S => CustomKeyCode::S,
        KeyCode::T => CustomKeyCode::T,
        KeyCode::U => CustomKeyCode::U,
        KeyCode::V => CustomKeyCode::V,
        KeyCode::W => CustomKeyCode::W,
        KeyCode::X => CustomKeyCode::X,
        KeyCode::Y => CustomKeyCode::Y,
        KeyCode::Z => CustomKeyCode::Z,
        KeyCode::Escape => CustomKeyCode::Escape,
        KeyCode::F1 => CustomKeyCode::F1,
        KeyCode::F2 => CustomKeyCode::F2,
        KeyCode::F3 => CustomKeyCode::F3,
        KeyCode::F4 => CustomKeyCode::F4,
        KeyCode::F5 => CustomKeyCode::F5,
        KeyCode::F6 => CustomKeyCode::F6,
        KeyCode::F7 => CustomKeyCode::F7,
        KeyCode::F8 => CustomKeyCode::F8,
        KeyCode::F9 => CustomKeyCode::F9,
        KeyCode::F10 => CustomKeyCode::F10,
        KeyCode::F11 => CustomKeyCode::F11,
        KeyCode::F12 => CustomKeyCode::F12,
        KeyCode::F13 => CustomKeyCode::F13,
        KeyCode::F14 => CustomKeyCode::F14,
        KeyCode::F15 => CustomKeyCode::F15,
        KeyCode::F16 => CustomKeyCode::F16,
        KeyCode::F17 => CustomKeyCode::F17,
        KeyCode::F18 => CustomKeyCode::F18,
        KeyCode::F19 => CustomKeyCode::F19,
        KeyCode::F20 => CustomKeyCode::F20,
        KeyCode::F21 => CustomKeyCode::F21,
        KeyCode::F22 => CustomKeyCode::F22,
        KeyCode::F23 => CustomKeyCode::F23,
        KeyCode::F24 => CustomKeyCode::F24,
        KeyCode::Snapshot => CustomKeyCode::Snapshot,
        KeyCode::Scroll => CustomKeyCode::Scroll,
        KeyCode::Pause => CustomKeyCode::Pause,
        KeyCode::Insert => CustomKeyCode::Insert,
        KeyCode::Home => CustomKeyCode::Home,
        KeyCode::Delete => CustomKeyCode::Delete,
        KeyCode::End => CustomKeyCode::End,
        KeyCode::PageDown => CustomKeyCode::PageDown,
        KeyCode::PageUp => CustomKeyCode::PageUp,
        KeyCode::Left => CustomKeyCode::Left,
        KeyCode::Up => CustomKeyCode::Up,
        KeyCode::Right => CustomKeyCode::Right,
        KeyCode::Down => CustomKeyCode::Down,
        KeyCode::Backspace => CustomKeyCode::Backspace,
        KeyCode::Enter => CustomKeyCode::Enter,
        KeyCode::Space => CustomKeyCode::Space,
        KeyCode::Compose => CustomKeyCode::Compose,
        KeyCode::Caret => CustomKeyCode::Caret,
        KeyCode::Numlock => CustomKeyCode::Numlock,
        KeyCode::Numpad0 => CustomKeyCode::Numpad0,
        KeyCode::Numpad1 => CustomKeyCode::Numpad1,
        KeyCode::Numpad2 => CustomKeyCode::Numpad2,
        KeyCode::Numpad3 => CustomKeyCode::Numpad3,
        KeyCode::Numpad4 => CustomKeyCode::Numpad4,
        KeyCode::Numpad5 => CustomKeyCode::Numpad5,
        KeyCode::Numpad6 => CustomKeyCode::Numpad6,
        KeyCode::Numpad7 => CustomKeyCode::Numpad7,
        KeyCode::Numpad8 => CustomKeyCode::Numpad8,
        KeyCode::Numpad9 => CustomKeyCode::Numpad9,
        KeyCode::NumpadAdd => CustomKeyCode::NumpadAdd,
        KeyCode::NumpadDivide => CustomKeyCode::NumpadDivide,
        KeyCode::NumpadDecimal => CustomKeyCode::NumpadDecimal,
        KeyCode::NumpadComma => CustomKeyCode::NumpadComma,
        KeyCode::NumpadEnter => CustomKeyCode::NumpadEnter,
        KeyCode::NumpadEquals => CustomKeyCode::NumpadEquals,
        KeyCode::NumpadMultiply => CustomKeyCode::NumpadMultiply,
        KeyCode::NumpadSubtract => CustomKeyCode::NumpadSubtract,
        KeyCode::AbntC1 => CustomKeyCode::AbntC1,
        KeyCode::AbntC2 => CustomKeyCode::AbntC2,
        KeyCode::Apostrophe => CustomKeyCode::Apostrophe,
        KeyCode::Apps => CustomKeyCode::Apps,
        KeyCode::Asterisk => CustomKeyCode::Asterisk,
        KeyCode::At => CustomKeyCode::At,
        KeyCode::Ax => CustomKeyCode::Ax,
        KeyCode::Backslash => CustomKeyCode::Backslash,
        KeyCode::Calculator => CustomKeyCode::Calculator,
        KeyCode::Capital => CustomKeyCode::Capital,
        KeyCode::Colon => CustomKeyCode::Colon,
        KeyCode::Comma => CustomKeyCode::Comma,
        KeyCode::Convert => CustomKeyCode::Convert,
        KeyCode::Equals => CustomKeyCode::Equals,
        KeyCode::Grave => CustomKeyCode::Grave,
        KeyCode::Kana => CustomKeyCode::Kana,
        KeyCode::Kanji => CustomKeyCode::Kanji,
        KeyCode::LAlt => CustomKeyCode::LAlt,
        KeyCode::LBracket => CustomKeyCode::LBracket,
        KeyCode::LControl => CustomKeyCode::LControl,
        KeyCode::LShift => CustomKeyCode::LShift,
        KeyCode::LWin => CustomKeyCode::LWin,
        KeyCode::Mail => CustomKeyCode::Mail,
        KeyCode::MediaSelect => CustomKeyCode::MediaSelect,
        KeyCode::MediaStop => CustomKeyCode::MediaStop,
        KeyCode::Minus => CustomKeyCode::Minus,
        KeyCode::Mute => CustomKeyCode::Mute,
        KeyCode::MyComputer => CustomKeyCode::MyComputer,
        KeyCode::NavigateForward => CustomKeyCode::NavigateForward,
        KeyCode::NavigateBackward => CustomKeyCode::NavigateBackward,
        KeyCode::NextTrack => CustomKeyCode::NextTrack,
        KeyCode::NoConvert => CustomKeyCode::NoConvert,
        KeyCode::OEM102 => CustomKeyCode::OEM102,
        KeyCode::Period => CustomKeyCode::Period,
        KeyCode::PlayPause => CustomKeyCode::PlayPause,
        KeyCode::Plus => CustomKeyCode::Plus,
        KeyCode::Power => CustomKeyCode::Power,
        KeyCode::PrevTrack => CustomKeyCode::PrevTrack,
        KeyCode::RAlt => CustomKeyCode::RAlt,
        KeyCode::RBracket => CustomKeyCode::RBracket,
        KeyCode::RControl => CustomKeyCode::RControl,
        KeyCode::RShift => CustomKeyCode::RShift,
        KeyCode::RWin => CustomKeyCode::RWin,
        KeyCode::Semicolon => CustomKeyCode::Semicolon,
        KeyCode::Slash => CustomKeyCode::Slash,
        KeyCode::Sleep => CustomKeyCode::Sleep,
        KeyCode::Stop => CustomKeyCode::Stop,
        KeyCode::Sysrq => CustomKeyCode::Sysrq,
        KeyCode::Tab => CustomKeyCode::Tab,
        KeyCode::Underline => CustomKeyCode::Underline,
        KeyCode::Unlabeled => CustomKeyCode::Unlabeled,
        KeyCode::VolumeDown => CustomKeyCode::VolumeDown,
        KeyCode::VolumeUp => CustomKeyCode::VolumeUp,
        KeyCode::Wake => CustomKeyCode::Wake,
        KeyCode::WebBack => CustomKeyCode::WebBack,
        KeyCode::WebFavorites => CustomKeyCode::WebFavorites,
        KeyCode::WebForward => CustomKeyCode::WebForward,
        KeyCode::WebHome => CustomKeyCode::WebHome,
        KeyCode::WebRefresh => CustomKeyCode::WebRefresh,
        KeyCode::WebSearch => CustomKeyCode::WebSearch,
        KeyCode::WebStop => CustomKeyCode::WebStop,
        KeyCode::Yen => CustomKeyCode::Yen,
        KeyCode::Copy => CustomKeyCode::Copy,
        KeyCode::Paste => CustomKeyCode::Paste,
        KeyCode::Cut => CustomKeyCode::Cut,
    }
}

pub fn get_iced_modifiers_from_custom_modifiers(modifiers: CustomModifiers) -> Modifiers {
    let mut iced_modifiers = Modifiers::default();
    if modifiers.shift() {
        iced_modifiers |= Modifiers::SHIFT;
    }
    if modifiers.control() {
        iced_modifiers |= Modifiers::CTRL;
    }
    if modifiers.alt() {
        iced_modifiers |= Modifiers::ALT;
    }
    if modifiers.logo() {
        iced_modifiers |= Modifiers::LOGO;
    }
    iced_modifiers
}

#[allow(dead_code)]
pub fn get_custom_modifiers_from_iced_modifiers(modifiers: Modifiers) -> CustomModifiers {
    let mut custom_modifiers = CustomModifiers::default();
    if modifiers.shift() {
        custom_modifiers |= CustomModifiers::SHIFT;
    }
    if modifiers.control() {
        custom_modifiers |= CustomModifiers::CTRL;
    }
    if modifiers.alt() {
        custom_modifiers |= CustomModifiers::ALT;
    }
    if modifiers.logo() {
        custom_modifiers |= CustomModifiers::LOGO;
    }
    custom_modifiers
}

pub fn get_readable_keybind_name(key: CustomKeyCode, modifiers: CustomModifiers) -> String {
    let mut name = String::new();

    if modifiers.control() {
        name.push_str("Ctrl+");
    }
    if modifiers.logo() {
        if cfg!(target_os = "macos") {
            name.push_str("Cmd+");
        } else {
            name.push_str("Win+");
        }
    };
    if modifiers.shift() {
        name.push_str("Shift+");
    }
    if modifiers.alt() {
        name.push_str("Alt+");
    }

    name.push_str(format!("{:?}", key).as_str());

    name
}
