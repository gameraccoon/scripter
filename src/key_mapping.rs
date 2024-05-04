// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use bitflags::bitflags;
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use serde::{Deserialize, Deserializer, Serialize};

use smol_str::SmolStr;

// this is pretty stupid code, but it solves a few problems:
// 1. we can serialize the keybinds
// 2. we can make sure if the keybinds enum changes the app won't
//    compile, so we can maake an updater for the old condifg
// 3. configs don't need to know about iced
// 4. we can give keybinds user friendly names

// UPD: Iced updated the Keybinds somewhere between 0.10 and 0.12.1, so this was actually
// helpful at that point to keep backward compatibility.
// Now unknown keys can be removed but the config needs to be updated for that
// new keys can be added at will without updating the config

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CustomKeyCode {
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

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
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
    NavigateForward,  // also called "Next"
    NavigateBackward, // also called "Prior"
    NextTrack,
    NoConvert,
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
    Stop,
    Tab,
    Underline,
    VolumeDown,
    VolumeUp,
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

    // Keys to rename
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
    Unlabeled,

    // Keys to remove
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
    Calculator,
    MyComputer,
    OEM102,
    Sleep,
    Sysrq,
    Wake,
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

pub fn get_iced_key_code_from_custom_key_code(key: CustomKeyCode) -> Key {
    match key {
        CustomKeyCode::Key1 => Key::Character(SmolStr::new_static("1")),
        CustomKeyCode::Key2 => Key::Character(SmolStr::new_static("2")),
        CustomKeyCode::Key3 => Key::Character(SmolStr::new_static("3")),
        CustomKeyCode::Key4 => Key::Character(SmolStr::new_static("4")),
        CustomKeyCode::Key5 => Key::Character(SmolStr::new_static("5")),
        CustomKeyCode::Key6 => Key::Character(SmolStr::new_static("6")),
        CustomKeyCode::Key7 => Key::Character(SmolStr::new_static("7")),
        CustomKeyCode::Key8 => Key::Character(SmolStr::new_static("8")),
        CustomKeyCode::Key9 => Key::Character(SmolStr::new_static("9")),
        CustomKeyCode::Key0 => Key::Character(SmolStr::new_static("0")),
        CustomKeyCode::A => Key::Character(SmolStr::new_static("a")),
        CustomKeyCode::B => Key::Character(SmolStr::new_static("b")),
        CustomKeyCode::C => Key::Character(SmolStr::new_static("c")),
        CustomKeyCode::D => Key::Character(SmolStr::new_static("d")),
        CustomKeyCode::E => Key::Character(SmolStr::new_static("e")),
        CustomKeyCode::F => Key::Character(SmolStr::new_static("f")),
        CustomKeyCode::G => Key::Character(SmolStr::new_static("g")),
        CustomKeyCode::H => Key::Character(SmolStr::new_static("h")),
        CustomKeyCode::I => Key::Character(SmolStr::new_static("i")),
        CustomKeyCode::J => Key::Character(SmolStr::new_static("j")),
        CustomKeyCode::K => Key::Character(SmolStr::new_static("k")),
        CustomKeyCode::L => Key::Character(SmolStr::new_static("l")),
        CustomKeyCode::M => Key::Character(SmolStr::new_static("m")),
        CustomKeyCode::N => Key::Character(SmolStr::new_static("n")),
        CustomKeyCode::O => Key::Character(SmolStr::new_static("o")),
        CustomKeyCode::P => Key::Character(SmolStr::new_static("p")),
        CustomKeyCode::Q => Key::Character(SmolStr::new_static("q")),
        CustomKeyCode::R => Key::Character(SmolStr::new_static("r")),
        CustomKeyCode::S => Key::Character(SmolStr::new_static("s")),
        CustomKeyCode::T => Key::Character(SmolStr::new_static("t")),
        CustomKeyCode::U => Key::Character(SmolStr::new_static("u")),
        CustomKeyCode::V => Key::Character(SmolStr::new_static("v")),
        CustomKeyCode::W => Key::Character(SmolStr::new_static("w")),
        CustomKeyCode::X => Key::Character(SmolStr::new_static("x")),
        CustomKeyCode::Y => Key::Character(SmolStr::new_static("y")),
        CustomKeyCode::Z => Key::Character(SmolStr::new_static("z")),
        CustomKeyCode::Escape => Key::Named(Named::Escape),
        CustomKeyCode::F1 => Key::Named(Named::F1),
        CustomKeyCode::F2 => Key::Named(Named::F2),
        CustomKeyCode::F3 => Key::Named(Named::F3),
        CustomKeyCode::F4 => Key::Named(Named::F4),
        CustomKeyCode::F5 => Key::Named(Named::F5),
        CustomKeyCode::F6 => Key::Named(Named::F6),
        CustomKeyCode::F7 => Key::Named(Named::F7),
        CustomKeyCode::F8 => Key::Named(Named::F8),
        CustomKeyCode::F9 => Key::Named(Named::F9),
        CustomKeyCode::F10 => Key::Named(Named::F10),
        CustomKeyCode::F11 => Key::Named(Named::F11),
        CustomKeyCode::F12 => Key::Named(Named::F12),
        CustomKeyCode::F13 => Key::Named(Named::F13),
        CustomKeyCode::F14 => Key::Named(Named::F14),
        CustomKeyCode::F15 => Key::Named(Named::F15),
        CustomKeyCode::F16 => Key::Named(Named::F16),
        CustomKeyCode::F17 => Key::Named(Named::F17),
        CustomKeyCode::F18 => Key::Named(Named::F18),
        CustomKeyCode::F19 => Key::Named(Named::F19),
        CustomKeyCode::F20 => Key::Named(Named::F20),
        CustomKeyCode::F21 => Key::Named(Named::F21),
        CustomKeyCode::F22 => Key::Named(Named::F22),
        CustomKeyCode::F23 => Key::Named(Named::F23),
        CustomKeyCode::F24 => Key::Named(Named::F24),
        CustomKeyCode::Snapshot => Key::Named(Named::PrintScreen),
        CustomKeyCode::Scroll => Key::Named(Named::ScrollLock),
        CustomKeyCode::Pause => Key::Named(Named::Pause),
        CustomKeyCode::Insert => Key::Named(Named::Insert),
        CustomKeyCode::Home => Key::Named(Named::Home),
        CustomKeyCode::Delete => Key::Named(Named::Delete),
        CustomKeyCode::End => Key::Named(Named::End),
        CustomKeyCode::PageDown => Key::Named(Named::PageDown),
        CustomKeyCode::PageUp => Key::Named(Named::PageUp),
        CustomKeyCode::Left => Key::Named(Named::ArrowLeft),
        CustomKeyCode::Up => Key::Named(Named::ArrowUp),
        CustomKeyCode::Right => Key::Named(Named::ArrowRight),
        CustomKeyCode::Down => Key::Named(Named::ArrowDown),
        CustomKeyCode::Backspace => Key::Named(Named::Backspace),
        CustomKeyCode::Enter => Key::Named(Named::Enter),
        CustomKeyCode::Space => Key::Named(Named::Space),
        CustomKeyCode::Compose => Key::Named(Named::Compose),
        CustomKeyCode::Caret => Key::Character(SmolStr::new_static("^")),
        CustomKeyCode::AbntC1 => Key::Character(SmolStr::new_static("`")),
        CustomKeyCode::AbntC2 => Key::Character(SmolStr::new_static("~")),
        CustomKeyCode::Apostrophe => Key::Character(SmolStr::new_static("\'")),
        CustomKeyCode::Apps => Key::Named(Named::ContextMenu),
        CustomKeyCode::Asterisk => Key::Character(SmolStr::new_static("*")),
        CustomKeyCode::At => Key::Character(SmolStr::new_static("@")),
        CustomKeyCode::Ax => Key::Character(SmolStr::new_static("`")),
        CustomKeyCode::Backslash => Key::Character(SmolStr::new_static("\\")),
        CustomKeyCode::Capital => Key::Named(Named::CapsLock),
        CustomKeyCode::Colon => Key::Character(SmolStr::new_static(":")),
        CustomKeyCode::Comma => Key::Character(SmolStr::new_static(",")),
        CustomKeyCode::Convert => Key::Named(Named::Convert),
        CustomKeyCode::Equals => Key::Character(SmolStr::new_static("=")),
        CustomKeyCode::Grave => Key::Character(SmolStr::new_static("`")),
        CustomKeyCode::Kana => Key::Named(Named::KanaMode),
        CustomKeyCode::Kanji => Key::Named(Named::KanjiMode),
        CustomKeyCode::LAlt => Key::Named(Named::Alt),
        CustomKeyCode::LBracket => Key::Character(SmolStr::new_static("[")),
        CustomKeyCode::LControl => Key::Named(Named::Control),
        CustomKeyCode::LShift => Key::Named(Named::Shift),
        CustomKeyCode::LWin => Key::Named(Named::Meta),
        CustomKeyCode::Mail => Key::Named(Named::LaunchMail),
        CustomKeyCode::MediaSelect => Key::Named(Named::LaunchMediaPlayer),
        CustomKeyCode::MediaStop => Key::Named(Named::MediaStop),
        CustomKeyCode::Minus => Key::Character(SmolStr::new_static("-")),
        CustomKeyCode::Mute => Key::Named(Named::AudioVolumeMute),
        CustomKeyCode::NavigateForward => Key::Named(Named::BrowserForward),
        CustomKeyCode::NavigateBackward => Key::Named(Named::BrowserBack),
        CustomKeyCode::NextTrack => Key::Named(Named::MediaTrackNext),
        CustomKeyCode::NoConvert => Key::Named(Named::NonConvert),
        CustomKeyCode::Period => Key::Character(SmolStr::new_static(".")),
        CustomKeyCode::PlayPause => Key::Named(Named::MediaPlayPause),
        CustomKeyCode::Plus => Key::Character(SmolStr::new_static("+")),
        CustomKeyCode::Power => Key::Named(Named::Power),
        CustomKeyCode::PrevTrack => Key::Named(Named::MediaTrackPrevious),
        CustomKeyCode::RAlt => Key::Named(Named::Alt),
        CustomKeyCode::RBracket => Key::Character(SmolStr::new_static("]")),
        CustomKeyCode::RControl => Key::Named(Named::Control),
        CustomKeyCode::RShift => Key::Named(Named::Shift),
        CustomKeyCode::RWin => Key::Named(Named::Meta),
        CustomKeyCode::Semicolon => Key::Character(SmolStr::new_static(";")),
        CustomKeyCode::Slash => Key::Character(SmolStr::new_static("/")),
        CustomKeyCode::Stop => Key::Named(Named::MediaStop),
        CustomKeyCode::Tab => Key::Named(Named::Tab),
        CustomKeyCode::Underline => Key::Character(SmolStr::new_static("_")),
        CustomKeyCode::VolumeDown => Key::Named(Named::AudioVolumeDown),
        CustomKeyCode::VolumeUp => Key::Named(Named::AudioVolumeUp),
        CustomKeyCode::WebBack => Key::Named(Named::BrowserBack),
        CustomKeyCode::WebFavorites => Key::Named(Named::BrowserFavorites),
        CustomKeyCode::WebForward => Key::Named(Named::BrowserForward),
        CustomKeyCode::WebHome => Key::Named(Named::BrowserHome),
        CustomKeyCode::WebRefresh => Key::Named(Named::BrowserRefresh),
        CustomKeyCode::WebSearch => Key::Named(Named::BrowserSearch),
        CustomKeyCode::WebStop => Key::Named(Named::BrowserStop),
        CustomKeyCode::Yen => Key::Character(SmolStr::new_static("¥")),
        CustomKeyCode::Copy => Key::Named(Named::Copy),
        CustomKeyCode::Paste => Key::Named(Named::Paste),
        CustomKeyCode::Cut => Key::Named(Named::Cut),

        CustomKeyCode::Unlabeled => Key::Unidentified,

        CustomKeyCode::Numlock => Key::Named(Named::NumLock),
        CustomKeyCode::Numpad0 => Key::Character(SmolStr::new_static("0")),
        CustomKeyCode::Numpad1 => Key::Character(SmolStr::new_static("1")),
        CustomKeyCode::Numpad2 => Key::Character(SmolStr::new_static("2")),
        CustomKeyCode::Numpad3 => Key::Character(SmolStr::new_static("3")),
        CustomKeyCode::Numpad4 => Key::Character(SmolStr::new_static("4")),
        CustomKeyCode::Numpad5 => Key::Character(SmolStr::new_static("5")),
        CustomKeyCode::Numpad6 => Key::Character(SmolStr::new_static("6")),
        CustomKeyCode::Numpad7 => Key::Character(SmolStr::new_static("7")),
        CustomKeyCode::Numpad8 => Key::Character(SmolStr::new_static("8")),
        CustomKeyCode::Numpad9 => Key::Character(SmolStr::new_static("9")),
        CustomKeyCode::NumpadAdd => Key::Character(SmolStr::new_static("+")),
        CustomKeyCode::NumpadDivide => Key::Character(SmolStr::new_static("/")),
        CustomKeyCode::NumpadDecimal => Key::Character(SmolStr::new_static(".")),
        CustomKeyCode::NumpadComma => Key::Character(SmolStr::new_static(",")),
        CustomKeyCode::NumpadEnter => Key::Named(Named::Enter),
        CustomKeyCode::NumpadEquals => Key::Character(SmolStr::new_static("=")),
        CustomKeyCode::NumpadMultiply => Key::Character(SmolStr::new_static("*")),
        CustomKeyCode::NumpadSubtract => Key::Character(SmolStr::new_static("-")),
        CustomKeyCode::Calculator => Key::Unidentified,
        CustomKeyCode::MyComputer => Key::Unidentified,
        CustomKeyCode::OEM102 => Key::Unidentified,
        CustomKeyCode::Sleep => Key::Unidentified,
        CustomKeyCode::Sysrq => Key::Unidentified,
        CustomKeyCode::Wake => Key::Unidentified,
    }
}

#[allow(dead_code)]
pub fn get_custom_key_code_from_iced_key_code(key: Key) -> CustomKeyCode {
    match key {
        Key::Character(char) => match char.as_str() {
            "1" => CustomKeyCode::Key1,
            "2" => CustomKeyCode::Key2,
            "3" => CustomKeyCode::Key3,
            "4" => CustomKeyCode::Key4,
            "5" => CustomKeyCode::Key5,
            "6" => CustomKeyCode::Key6,
            "7" => CustomKeyCode::Key7,
            "8" => CustomKeyCode::Key8,
            "9" => CustomKeyCode::Key9,
            "0" => CustomKeyCode::Key0,
            "a" => CustomKeyCode::A,
            "b" => CustomKeyCode::B,
            "c" => CustomKeyCode::C,
            "d" => CustomKeyCode::D,
            "e" => CustomKeyCode::E,
            "f" => CustomKeyCode::F,
            "g" => CustomKeyCode::G,
            "h" => CustomKeyCode::H,
            "i" => CustomKeyCode::I,
            "j" => CustomKeyCode::J,
            "k" => CustomKeyCode::K,
            "l" => CustomKeyCode::L,
            "m" => CustomKeyCode::M,
            "n" => CustomKeyCode::N,
            "o" => CustomKeyCode::O,
            "p" => CustomKeyCode::P,
            "q" => CustomKeyCode::Q,
            "r" => CustomKeyCode::R,
            "s" => CustomKeyCode::S,
            "t" => CustomKeyCode::T,
            "u" => CustomKeyCode::U,
            "v" => CustomKeyCode::V,
            "w" => CustomKeyCode::W,
            "x" => CustomKeyCode::X,
            "y" => CustomKeyCode::Y,
            "z" => CustomKeyCode::Z,
            "^" => CustomKeyCode::Caret,
            "." => CustomKeyCode::Period,
            "," => CustomKeyCode::Comma,
            "+" => CustomKeyCode::Plus,
            "-" => CustomKeyCode::Minus,
            "/" => CustomKeyCode::Slash,
            "*" => CustomKeyCode::Asterisk,
            "=" => CustomKeyCode::Equals,
            ";" => CustomKeyCode::Semicolon,
            ":" => CustomKeyCode::Colon,
            "_" => CustomKeyCode::Underline,
            "\\" => CustomKeyCode::Backslash,
            "@" => CustomKeyCode::At,
            "`" => CustomKeyCode::AbntC1,
            "~" => CustomKeyCode::AbntC2,
            "'" => CustomKeyCode::Apostrophe,
            "¥" => CustomKeyCode::Yen,
            _ => {
                println!("Unknown key: {}", char);
                CustomKeyCode::Unlabeled
            }
        },
        Key::Named(named) => match named {
            Named::Escape => CustomKeyCode::Escape,
            Named::F1 => CustomKeyCode::F1,
            Named::F2 => CustomKeyCode::F2,
            Named::F3 => CustomKeyCode::F3,
            Named::F4 => CustomKeyCode::F4,
            Named::F5 => CustomKeyCode::F5,
            Named::F6 => CustomKeyCode::F6,
            Named::F7 => CustomKeyCode::F7,
            Named::F8 => CustomKeyCode::F8,
            Named::F9 => CustomKeyCode::F9,
            Named::F10 => CustomKeyCode::F10,
            Named::F11 => CustomKeyCode::F11,
            Named::F12 => CustomKeyCode::F12,
            Named::F13 => CustomKeyCode::F13,
            Named::F14 => CustomKeyCode::F14,
            Named::F15 => CustomKeyCode::F15,
            Named::F16 => CustomKeyCode::F16,
            Named::F17 => CustomKeyCode::F17,
            Named::F18 => CustomKeyCode::F18,
            Named::F19 => CustomKeyCode::F19,
            Named::F20 => CustomKeyCode::F20,
            Named::F21 => CustomKeyCode::F21,
            Named::F22 => CustomKeyCode::F22,
            Named::F23 => CustomKeyCode::F23,
            Named::F24 => CustomKeyCode::F24,
            Named::PrintScreen => CustomKeyCode::Snapshot,
            Named::ScrollLock => CustomKeyCode::Scroll,
            Named::Pause => CustomKeyCode::Pause,
            Named::Insert => CustomKeyCode::Insert,
            Named::Home => CustomKeyCode::Home,
            Named::Delete => CustomKeyCode::Delete,
            Named::End => CustomKeyCode::End,
            Named::PageDown => CustomKeyCode::PageDown,
            Named::PageUp => CustomKeyCode::PageUp,
            Named::ArrowLeft => CustomKeyCode::Left,
            Named::ArrowUp => CustomKeyCode::Up,
            Named::ArrowRight => CustomKeyCode::Right,
            Named::ArrowDown => CustomKeyCode::Down,
            Named::Backspace => CustomKeyCode::Backspace,
            Named::Enter => CustomKeyCode::Enter,
            Named::Space => CustomKeyCode::Space,
            Named::Compose => CustomKeyCode::Compose,
            Named::NumLock => CustomKeyCode::Numlock,
            Named::ContextMenu => CustomKeyCode::Apps,
            Named::CapsLock => CustomKeyCode::Capital,
            Named::KanaMode => CustomKeyCode::Kana,
            Named::KanjiMode => CustomKeyCode::Kanji,
            Named::Alt => CustomKeyCode::LAlt,
            Named::Control => CustomKeyCode::LControl,
            Named::Shift => CustomKeyCode::LShift,
            Named::Meta => CustomKeyCode::LWin,
            Named::LaunchMail => CustomKeyCode::Mail,
            Named::LaunchMediaPlayer => CustomKeyCode::MediaSelect,
            Named::MediaStop => CustomKeyCode::MediaStop,
            Named::AudioVolumeMute => CustomKeyCode::Mute,
            Named::BrowserForward => CustomKeyCode::NavigateForward,
            Named::BrowserBack => CustomKeyCode::NavigateBackward,
            Named::MediaTrackNext => CustomKeyCode::NextTrack,
            Named::NonConvert => CustomKeyCode::NoConvert,
            Named::Power => CustomKeyCode::Power,
            Named::MediaTrackPrevious => CustomKeyCode::PrevTrack,
            Named::Tab => CustomKeyCode::Tab,
            Named::AudioVolumeDown => CustomKeyCode::VolumeDown,
            Named::AudioVolumeUp => CustomKeyCode::VolumeUp,
            Named::BrowserFavorites => CustomKeyCode::WebFavorites,
            Named::BrowserHome => CustomKeyCode::WebHome,
            Named::BrowserRefresh => CustomKeyCode::WebRefresh,
            Named::BrowserSearch => CustomKeyCode::WebSearch,
            Named::BrowserStop => CustomKeyCode::WebStop,
            Named::Copy => CustomKeyCode::Copy,
            Named::Paste => CustomKeyCode::Paste,
            Named::Cut => CustomKeyCode::Cut,
            _ => CustomKeyCode::Unlabeled,
        },
        _ => CustomKeyCode::Unlabeled,
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
