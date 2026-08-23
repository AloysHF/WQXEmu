// Model-specific keyboard layouts.
//
// Every Wenquxing model exposes an 8x8 keypad matrix, but the physical
// key arrangement (and therefore which PC key maps to which matrix
// position) differs per model. This module is the single source of
// truth for the frontends: it describes the on-screen keys (label, PC
// key hint) for each supported model so the desktop frontend can draw a
// virtual keypad and accept mouse clicks.

use std::collections::HashSet;

use crate::input::KEY_COUNT;
use crate::machine::MachineModel;

/// A frontend-independent physical keyboard key.
///
/// Frontends translate their native key codes into this type so every
/// frontend uses the same Wenquxing keypad mapping.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HostKey {
    Letter(char),
    Digit(u8),
    Function(u8),
    Return,
    Escape,
    Space,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
}

/// Combined frontend input state for keyboard, controller, and pointer input.
#[derive(Clone, Debug)]
pub struct FrontendInputState {
    keyboard: HashSet<HostKey>,
    controller: HashSet<HostKey>,
    pointer: Option<u8>,
    applied: [bool; KEY_COUNT],
}

impl Default for FrontendInputState {
    fn default() -> Self {
        Self {
            keyboard: HashSet::new(),
            controller: HashSet::new(),
            pointer: None,
            applied: [false; KEY_COUNT],
        }
    }
}

impl FrontendInputState {
    pub fn set_keyboard_key(&mut self, key: HostKey, pressed: bool) {
        if pressed {
            self.keyboard.insert(key);
        } else {
            self.keyboard.remove(&key);
        }
    }

    pub fn set_controller_key(&mut self, key: HostKey, pressed: bool) {
        if pressed {
            self.controller.insert(key);
        } else {
            self.controller.remove(&key);
        }
    }

    pub fn set_pointer_key(&mut self, key_id: Option<u8>) {
        self.pointer = key_id.filter(|key_id| (*key_id as usize) < KEY_COUNT);
    }

    /// Forget the previously applied matrix after the emulated machine resets.
    pub fn reset_applied(&mut self) {
        self.applied.fill(false);
    }

    /// Release every emulated key after loading state that may contain input.
    pub fn release_all(&mut self, mut set_key: impl FnMut(u8, bool)) {
        for key_id in 0..KEY_COUNT {
            set_key(key_id as u8, false);
        }
        self.reset_applied();
    }

    /// Release all frontend inputs and forget the applied matrix.
    pub fn clear(&mut self) {
        self.keyboard.clear();
        self.controller.clear();
        self.pointer = None;
        self.reset_applied();
    }

    pub fn pressed(&self) -> &[bool; KEY_COUNT] {
        &self.applied
    }

    /// Merge every input source and emit only keypad matrix changes.
    pub fn sync(&mut self, model: MachineModel, mut set_key: impl FnMut(u8, bool)) {
        let mut desired = [false; KEY_COUNT];
        for host_key in self.keyboard.iter().chain(&self.controller) {
            if let Some(key_id) = key_id_for_host_key(model, *host_key) {
                desired[key_id as usize] = true;
            }
        }
        if let Some(key_id) = self.pointer {
            desired[key_id as usize] = true;
        }

        for (key_id, (&next, &current)) in desired.iter().zip(&self.applied).enumerate() {
            if next != current {
                set_key(key_id as u8, next);
            }
        }
        self.applied = desired;
    }
}

/// A single on-screen key.
///
/// `drow`/`dcol` are the physical display position (6 rows x 10 cols,
/// matching the real Wenquxing keypad), while `row`/`col` are the keypad
/// matrix coordinates used for input.
#[derive(Clone, Copy, Debug)]
pub struct KeyDef {
    /// Physical display row (0-5).
    pub drow: u8,
    /// Physical display column (0-9).
    pub dcol: u8,
    /// Keypad matrix row (0-7).
    pub row: u8,
    /// Keypad matrix column (0-7).
    pub col: u8,
    /// Key-face label shown on the virtual keypad (ASCII).
    pub label: &'static str,
    /// PC key hint (e.g. "F5", "A", "UP"); empty when unmapped.
    pub hint: &'static str,
}

/// Key ID for a matrix position in the machine encoding.
///
/// Most models encode `key_id = row << 3 | col`, but the NC1020 family
/// (following the NC1020 reference implementation) encodes the opposite
/// way: `row = key_id % 8`, `col = key_id / 8`.
pub fn key_id_for(model: MachineModel, row: u8, col: u8) -> u8 {
    match model {
        MachineModel::Nc1020 => (col << 3) | row,
        _ => (row << 3) | col,
    }
}

fn key_has_alias(key: &KeyDef, expected: &str) -> bool {
    key.label
        .split('/')
        .chain(key.hint.split('/'))
        .any(|alias| alias == expected)
}

/// Resolve a physical keyboard key to the active model's keypad matrix.
pub fn key_id_for_host_key(model: MachineModel, host_key: HostKey) -> Option<u8> {
    let fixed_alias = match host_key {
        HostKey::Return => Some("ENT"),
        HostKey::Escape => Some("ESC"),
        HostKey::Space => Some("SPC"),
        HostKey::Backspace => Some("F2"),
        HostKey::Delete | HostKey::Function(12) if model == MachineModel::Nc1020 => Some("DEL"),
        HostKey::Delete => Some("F12"),
        HostKey::Up => Some("UP"),
        HostKey::Down => Some("DN"),
        HostKey::Left => Some("LT"),
        HostKey::Right => Some("RT"),
        HostKey::PageUp => Some("PGUP"),
        HostKey::PageDown => Some("PGDN"),
        HostKey::Letter(_) | HostKey::Digit(_) | HostKey::Function(_) => None,
    };

    layout_for(model)
        .iter()
        .find(|key| match host_key {
            HostKey::Function(12) if model == MachineModel::Nc1020 => key_has_alias(key, "DEL"),
            HostKey::Letter(letter) if letter.is_ascii_alphabetic() => {
                let expected = letter.to_ascii_uppercase();
                key.label
                    .split('/')
                    .chain(key.hint.split('/'))
                    .any(|alias| alias.len() == 1 && alias.starts_with(expected))
            }
            HostKey::Digit(digit) if digit <= 9 => {
                let expected = char::from(b'0' + digit);
                key.label
                    .split('/')
                    .chain(key.hint.split('/'))
                    .any(|alias| alias.len() == 1 && alias.starts_with(expected))
            }
            HostKey::Function(number) if (1..=12).contains(&number) => key
                .label
                .split('/')
                .chain(key.hint.split('/'))
                .any(|alias| {
                    alias
                        .strip_prefix('F')
                        .and_then(|value| value.parse::<u8>().ok())
                        == Some(number)
                }),
            _ => fixed_alias.is_some_and(|alias| key_has_alias(key, alias)),
        })
        .map(|key| key_id_for(model, key.row, key.col))
}

/// NC2000 keypad: standard 6x10 keypad with hotkeys in matrix column 1.
const LAYOUT_NC2000: &[KeyDef] = &[
    // Top row: hotkeys (英汉 名片 计算 行程 资料 时间 网络).
    KeyDef {
        drow: 0,
        dcol: 0,
        row: 3,
        col: 1,
        label: "DICT",
        hint: "F5",
    },
    KeyDef {
        drow: 0,
        dcol: 1,
        row: 4,
        col: 1,
        label: "CARD",
        hint: "F6",
    },
    KeyDef {
        drow: 0,
        dcol: 2,
        row: 5,
        col: 1,
        label: "CALC",
        hint: "F7",
    },
    KeyDef {
        drow: 0,
        dcol: 3,
        row: 2,
        col: 1,
        label: "MEMO",
        hint: "F8",
    },
    KeyDef {
        drow: 0,
        dcol: 4,
        row: 1,
        col: 1,
        label: "DATA",
        hint: "F9",
    },
    KeyDef {
        drow: 0,
        dcol: 5,
        row: 0,
        col: 1,
        label: "TIME",
        hint: "F10",
    },
    KeyDef {
        drow: 0,
        dcol: 6,
        row: 6,
        col: 1,
        label: "NET",
        hint: "F11",
    },
    // Function keys + power.
    KeyDef {
        drow: 1,
        dcol: 2,
        row: 0,
        col: 2,
        label: "F1",
        hint: "F1",
    },
    KeyDef {
        drow: 1,
        dcol: 3,
        row: 1,
        col: 2,
        label: "F2",
        hint: "F2",
    },
    KeyDef {
        drow: 1,
        dcol: 4,
        row: 2,
        col: 2,
        label: "F3",
        hint: "F3",
    },
    KeyDef {
        drow: 1,
        dcol: 5,
        row: 3,
        col: 2,
        label: "F4",
        hint: "F4",
    },
    KeyDef {
        drow: 1,
        dcol: 8,
        row: 0,
        col: 0,
        label: "ON",
        hint: "F12",
    },
    // QWERTY rows (matrix columns 4,5,6 and 3).
    KeyDef {
        drow: 2,
        dcol: 0,
        row: 0,
        col: 4,
        label: "Q",
        hint: "Q",
    },
    KeyDef {
        drow: 2,
        dcol: 1,
        row: 1,
        col: 4,
        label: "W",
        hint: "W",
    },
    KeyDef {
        drow: 2,
        dcol: 2,
        row: 2,
        col: 4,
        label: "E",
        hint: "E",
    },
    KeyDef {
        drow: 2,
        dcol: 3,
        row: 3,
        col: 4,
        label: "R",
        hint: "R",
    },
    KeyDef {
        drow: 2,
        dcol: 4,
        row: 4,
        col: 4,
        label: "T",
        hint: "T/7",
    },
    KeyDef {
        drow: 2,
        dcol: 5,
        row: 5,
        col: 4,
        label: "Y",
        hint: "Y/8",
    },
    KeyDef {
        drow: 2,
        dcol: 6,
        row: 6,
        col: 4,
        label: "U",
        hint: "U/9",
    },
    KeyDef {
        drow: 2,
        dcol: 7,
        row: 7,
        col: 4,
        label: "I",
        hint: "I",
    },
    KeyDef {
        drow: 2,
        dcol: 8,
        row: 0,
        col: 3,
        label: "O",
        hint: "O",
    },
    KeyDef {
        drow: 2,
        dcol: 9,
        row: 4,
        col: 3,
        label: "P",
        hint: "P",
    },
    KeyDef {
        drow: 3,
        dcol: 0,
        row: 0,
        col: 5,
        label: "A",
        hint: "A",
    },
    KeyDef {
        drow: 3,
        dcol: 1,
        row: 1,
        col: 5,
        label: "S",
        hint: "S",
    },
    KeyDef {
        drow: 3,
        dcol: 2,
        row: 2,
        col: 5,
        label: "D",
        hint: "D",
    },
    KeyDef {
        drow: 3,
        dcol: 3,
        row: 3,
        col: 5,
        label: "F",
        hint: "F",
    },
    KeyDef {
        drow: 3,
        dcol: 4,
        row: 4,
        col: 5,
        label: "G",
        hint: "G/4",
    },
    KeyDef {
        drow: 3,
        dcol: 5,
        row: 5,
        col: 5,
        label: "H",
        hint: "H/5",
    },
    KeyDef {
        drow: 3,
        dcol: 6,
        row: 6,
        col: 5,
        label: "J",
        hint: "J/6",
    },
    KeyDef {
        drow: 3,
        dcol: 7,
        row: 7,
        col: 5,
        label: "K",
        hint: "K",
    },
    KeyDef {
        drow: 3,
        dcol: 8,
        row: 1,
        col: 3,
        label: "L",
        hint: "L",
    },
    KeyDef {
        drow: 3,
        dcol: 9,
        row: 5,
        col: 3,
        label: "ENT",
        hint: "ENT",
    },
    KeyDef {
        drow: 4,
        dcol: 0,
        row: 0,
        col: 6,
        label: "Z",
        hint: "Z",
    },
    KeyDef {
        drow: 4,
        dcol: 1,
        row: 1,
        col: 6,
        label: "X",
        hint: "X",
    },
    KeyDef {
        drow: 4,
        dcol: 2,
        row: 2,
        col: 6,
        label: "C",
        hint: "C",
    },
    KeyDef {
        drow: 4,
        dcol: 3,
        row: 3,
        col: 6,
        label: "V",
        hint: "V",
    },
    KeyDef {
        drow: 4,
        dcol: 4,
        row: 4,
        col: 6,
        label: "B",
        hint: "B/1",
    },
    KeyDef {
        drow: 4,
        dcol: 5,
        row: 5,
        col: 6,
        label: "N",
        hint: "N/2",
    },
    KeyDef {
        drow: 4,
        dcol: 6,
        row: 6,
        col: 6,
        label: "M",
        hint: "M/3",
    },
    KeyDef {
        drow: 4,
        dcol: 7,
        row: 7,
        col: 6,
        label: "PGUP",
        hint: "PGUP",
    },
    KeyDef {
        drow: 4,
        dcol: 8,
        row: 2,
        col: 3,
        label: "UP",
        hint: "UP",
    },
    KeyDef {
        drow: 4,
        dcol: 9,
        row: 6,
        col: 3,
        label: "PGDN",
        hint: "PGDN",
    },
    // Bottom row.
    KeyDef {
        drow: 5,
        dcol: 0,
        row: 0,
        col: 7,
        label: "HELP",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 1,
        row: 1,
        col: 7,
        label: "SHIFT",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 2,
        row: 2,
        col: 7,
        label: "IME",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 3,
        row: 3,
        col: 7,
        label: "ESC",
        hint: "ESC",
    },
    KeyDef {
        drow: 5,
        dcol: 4,
        row: 4,
        col: 7,
        label: "0",
        hint: "0",
    },
    KeyDef {
        drow: 5,
        dcol: 5,
        row: 5,
        col: 7,
        label: ".",
        hint: ".",
    },
    KeyDef {
        drow: 5,
        dcol: 6,
        row: 6,
        col: 7,
        label: "SPC",
        hint: "SPC",
    },
    KeyDef {
        drow: 5,
        dcol: 7,
        row: 7,
        col: 7,
        label: "LT",
        hint: "LT",
    },
    KeyDef {
        drow: 5,
        dcol: 8,
        row: 3,
        col: 3,
        label: "DN",
        hint: "DN",
    },
    KeyDef {
        drow: 5,
        dcol: 9,
        row: 7,
        col: 3,
        label: "RT",
        hint: "RT",
    },
];

/// NC3000 keypad: standard 6x10 keypad with hotkeys in matrix column 0.
const LAYOUT_NC3000: &[KeyDef] = &[
    // Top row: 游戏 计算 时间 英汉 词库 学习.
    KeyDef {
        drow: 0,
        dcol: 0,
        row: 1,
        col: 0,
        label: "GAME",
        hint: "F5",
    },
    KeyDef {
        drow: 0,
        dcol: 1,
        row: 2,
        col: 0,
        label: "CALC",
        hint: "F6",
    },
    KeyDef {
        drow: 0,
        dcol: 2,
        row: 3,
        col: 0,
        label: "TIME",
        hint: "F7",
    },
    KeyDef {
        drow: 0,
        dcol: 3,
        row: 5,
        col: 0,
        label: "DICT",
        hint: "F9",
    },
    KeyDef {
        drow: 0,
        dcol: 4,
        row: 6,
        col: 0,
        label: "LEX",
        hint: "F10",
    },
    KeyDef {
        drow: 0,
        dcol: 5,
        row: 7,
        col: 0,
        label: "STUDY",
        hint: "F11",
    },
    // Function keys + power (网络/电源).
    KeyDef {
        drow: 1,
        dcol: 2,
        row: 0,
        col: 2,
        label: "F1",
        hint: "F1",
    },
    KeyDef {
        drow: 1,
        dcol: 3,
        row: 1,
        col: 2,
        label: "F2",
        hint: "F2",
    },
    KeyDef {
        drow: 1,
        dcol: 4,
        row: 2,
        col: 2,
        label: "F3",
        hint: "F3",
    },
    KeyDef {
        drow: 1,
        dcol: 5,
        row: 3,
        col: 2,
        label: "F4",
        hint: "F4",
    },
    KeyDef {
        drow: 1,
        dcol: 8,
        row: 0,
        col: 0,
        label: "PWR",
        hint: "F12",
    },
    // QWERTY rows (matrix columns 4,5,6 and 3).
    KeyDef {
        drow: 2,
        dcol: 0,
        row: 0,
        col: 4,
        label: "Q",
        hint: "Q",
    },
    KeyDef {
        drow: 2,
        dcol: 1,
        row: 1,
        col: 4,
        label: "W",
        hint: "W",
    },
    KeyDef {
        drow: 2,
        dcol: 2,
        row: 2,
        col: 4,
        label: "E",
        hint: "E",
    },
    KeyDef {
        drow: 2,
        dcol: 3,
        row: 3,
        col: 4,
        label: "R",
        hint: "R",
    },
    KeyDef {
        drow: 2,
        dcol: 4,
        row: 4,
        col: 4,
        label: "T",
        hint: "T/7",
    },
    KeyDef {
        drow: 2,
        dcol: 5,
        row: 5,
        col: 4,
        label: "Y",
        hint: "Y/8",
    },
    KeyDef {
        drow: 2,
        dcol: 6,
        row: 6,
        col: 4,
        label: "U",
        hint: "U/9",
    },
    KeyDef {
        drow: 2,
        dcol: 7,
        row: 7,
        col: 4,
        label: "I",
        hint: "I",
    },
    KeyDef {
        drow: 2,
        dcol: 8,
        row: 0,
        col: 3,
        label: "O",
        hint: "O",
    },
    KeyDef {
        drow: 2,
        dcol: 9,
        row: 4,
        col: 3,
        label: "P",
        hint: "P",
    },
    KeyDef {
        drow: 3,
        dcol: 0,
        row: 0,
        col: 5,
        label: "A",
        hint: "A",
    },
    KeyDef {
        drow: 3,
        dcol: 1,
        row: 1,
        col: 5,
        label: "S",
        hint: "S",
    },
    KeyDef {
        drow: 3,
        dcol: 2,
        row: 2,
        col: 5,
        label: "D",
        hint: "D",
    },
    KeyDef {
        drow: 3,
        dcol: 3,
        row: 3,
        col: 5,
        label: "F",
        hint: "F",
    },
    KeyDef {
        drow: 3,
        dcol: 4,
        row: 4,
        col: 5,
        label: "G",
        hint: "G/4",
    },
    KeyDef {
        drow: 3,
        dcol: 5,
        row: 5,
        col: 5,
        label: "H",
        hint: "H/5",
    },
    KeyDef {
        drow: 3,
        dcol: 6,
        row: 6,
        col: 5,
        label: "J",
        hint: "J/6",
    },
    KeyDef {
        drow: 3,
        dcol: 7,
        row: 7,
        col: 5,
        label: "K",
        hint: "K",
    },
    KeyDef {
        drow: 3,
        dcol: 8,
        row: 1,
        col: 3,
        label: "L",
        hint: "L",
    },
    KeyDef {
        drow: 3,
        dcol: 9,
        row: 5,
        col: 3,
        label: "ENT",
        hint: "ENT",
    },
    KeyDef {
        drow: 4,
        dcol: 0,
        row: 0,
        col: 6,
        label: "Z",
        hint: "Z",
    },
    KeyDef {
        drow: 4,
        dcol: 1,
        row: 1,
        col: 6,
        label: "X",
        hint: "X",
    },
    KeyDef {
        drow: 4,
        dcol: 2,
        row: 2,
        col: 6,
        label: "C",
        hint: "C",
    },
    KeyDef {
        drow: 4,
        dcol: 3,
        row: 3,
        col: 6,
        label: "V",
        hint: "V",
    },
    KeyDef {
        drow: 4,
        dcol: 4,
        row: 4,
        col: 6,
        label: "B",
        hint: "B/1",
    },
    KeyDef {
        drow: 4,
        dcol: 5,
        row: 5,
        col: 6,
        label: "N",
        hint: "N/2",
    },
    KeyDef {
        drow: 4,
        dcol: 6,
        row: 6,
        col: 6,
        label: "M",
        hint: "M/3",
    },
    KeyDef {
        drow: 4,
        dcol: 7,
        row: 7,
        col: 6,
        label: "PGUP",
        hint: "PGUP",
    },
    KeyDef {
        drow: 4,
        dcol: 8,
        row: 2,
        col: 3,
        label: "UP",
        hint: "UP",
    },
    KeyDef {
        drow: 4,
        dcol: 9,
        row: 6,
        col: 3,
        label: "PGDN",
        hint: "PGDN",
    },
    // Bottom row.
    KeyDef {
        drow: 5,
        dcol: 0,
        row: 0,
        col: 7,
        label: "HELP",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 1,
        row: 1,
        col: 7,
        label: "SHIFT",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 2,
        row: 2,
        col: 7,
        label: "IME",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 3,
        row: 3,
        col: 7,
        label: "ESC",
        hint: "ESC",
    },
    KeyDef {
        drow: 5,
        dcol: 4,
        row: 4,
        col: 7,
        label: "0",
        hint: "0",
    },
    KeyDef {
        drow: 5,
        dcol: 5,
        row: 5,
        col: 7,
        label: ".",
        hint: ".",
    },
    KeyDef {
        drow: 5,
        dcol: 6,
        row: 6,
        col: 7,
        label: "SPC",
        hint: "SPC",
    },
    KeyDef {
        drow: 5,
        dcol: 7,
        row: 7,
        col: 7,
        label: "LT",
        hint: "LT",
    },
    KeyDef {
        drow: 5,
        dcol: 8,
        row: 3,
        col: 3,
        label: "DN",
        hint: "DN",
    },
    KeyDef {
        drow: 5,
        dcol: 9,
        row: 7,
        col: 3,
        label: "RT",
        hint: "RT",
    },
];

/// PC1000 / CC800 keypad (same matrix, from the PC1000 reference keymap).
const LAYOUT_PC1000: &[KeyDef] = &[
    // Top row: hotkeys.
    KeyDef {
        drow: 0,
        dcol: 0,
        row: 1,
        col: 0,
        label: "DICT",
        hint: "F5",
    },
    KeyDef {
        drow: 0,
        dcol: 1,
        row: 1,
        col: 1,
        label: "CARD",
        hint: "F6",
    },
    KeyDef {
        drow: 0,
        dcol: 2,
        row: 1,
        col: 2,
        label: "CALC",
        hint: "F7",
    },
    KeyDef {
        drow: 0,
        dcol: 3,
        row: 1,
        col: 3,
        label: "MEMO",
        hint: "F8",
    },
    KeyDef {
        drow: 0,
        dcol: 4,
        row: 1,
        col: 4,
        label: "DATA",
        hint: "F9",
    },
    KeyDef {
        drow: 0,
        dcol: 5,
        row: 1,
        col: 5,
        label: "TIME",
        hint: "F10",
    },
    KeyDef {
        drow: 0,
        dcol: 6,
        row: 1,
        col: 6,
        label: "NET",
        hint: "F11",
    },
    // Function keys + power.
    KeyDef {
        drow: 1,
        dcol: 2,
        row: 7,
        col: 2,
        label: "F1",
        hint: "F1",
    },
    KeyDef {
        drow: 1,
        dcol: 3,
        row: 7,
        col: 3,
        label: "F2",
        hint: "F2",
    },
    KeyDef {
        drow: 1,
        dcol: 4,
        row: 7,
        col: 4,
        label: "F3",
        hint: "F3",
    },
    KeyDef {
        drow: 1,
        dcol: 5,
        row: 7,
        col: 5,
        label: "F4",
        hint: "F4",
    },
    KeyDef {
        drow: 1,
        dcol: 8,
        row: 0,
        col: 0,
        label: "ON",
        hint: "F12",
    },
    // QWERTY rows.
    KeyDef {
        drow: 2,
        dcol: 0,
        row: 5,
        col: 0,
        label: "Q",
        hint: "Q",
    },
    KeyDef {
        drow: 2,
        dcol: 1,
        row: 5,
        col: 1,
        label: "W",
        hint: "W",
    },
    KeyDef {
        drow: 2,
        dcol: 2,
        row: 5,
        col: 2,
        label: "E",
        hint: "E",
    },
    KeyDef {
        drow: 2,
        dcol: 3,
        row: 5,
        col: 3,
        label: "R",
        hint: "R",
    },
    KeyDef {
        drow: 2,
        dcol: 4,
        row: 5,
        col: 4,
        label: "T",
        hint: "T/7",
    },
    KeyDef {
        drow: 2,
        dcol: 5,
        row: 5,
        col: 5,
        label: "Y",
        hint: "Y/8",
    },
    KeyDef {
        drow: 2,
        dcol: 6,
        row: 5,
        col: 6,
        label: "U",
        hint: "U/9",
    },
    KeyDef {
        drow: 2,
        dcol: 7,
        row: 5,
        col: 7,
        label: "I",
        hint: "I",
    },
    KeyDef {
        drow: 2,
        dcol: 8,
        row: 6,
        col: 0,
        label: "O",
        hint: "O",
    },
    KeyDef {
        drow: 2,
        dcol: 9,
        row: 6,
        col: 4,
        label: "P",
        hint: "P",
    },
    KeyDef {
        drow: 3,
        dcol: 0,
        row: 4,
        col: 0,
        label: "A",
        hint: "A",
    },
    KeyDef {
        drow: 3,
        dcol: 1,
        row: 4,
        col: 1,
        label: "S",
        hint: "S",
    },
    KeyDef {
        drow: 3,
        dcol: 2,
        row: 4,
        col: 2,
        label: "D",
        hint: "D",
    },
    KeyDef {
        drow: 3,
        dcol: 3,
        row: 4,
        col: 3,
        label: "F",
        hint: "F",
    },
    KeyDef {
        drow: 3,
        dcol: 4,
        row: 4,
        col: 4,
        label: "G",
        hint: "G/4",
    },
    KeyDef {
        drow: 3,
        dcol: 5,
        row: 4,
        col: 5,
        label: "H",
        hint: "H/5",
    },
    KeyDef {
        drow: 3,
        dcol: 6,
        row: 4,
        col: 6,
        label: "J",
        hint: "J/6",
    },
    KeyDef {
        drow: 3,
        dcol: 7,
        row: 4,
        col: 7,
        label: "K",
        hint: "K",
    },
    KeyDef {
        drow: 3,
        dcol: 8,
        row: 6,
        col: 1,
        label: "L",
        hint: "L",
    },
    KeyDef {
        drow: 3,
        dcol: 9,
        row: 6,
        col: 5,
        label: "ENT",
        hint: "ENT",
    },
    KeyDef {
        drow: 4,
        dcol: 0,
        row: 3,
        col: 0,
        label: "Z",
        hint: "Z",
    },
    KeyDef {
        drow: 4,
        dcol: 1,
        row: 3,
        col: 1,
        label: "X",
        hint: "X",
    },
    KeyDef {
        drow: 4,
        dcol: 2,
        row: 3,
        col: 2,
        label: "C",
        hint: "C",
    },
    KeyDef {
        drow: 4,
        dcol: 3,
        row: 3,
        col: 3,
        label: "V",
        hint: "V",
    },
    KeyDef {
        drow: 4,
        dcol: 4,
        row: 3,
        col: 4,
        label: "B",
        hint: "B/1",
    },
    KeyDef {
        drow: 4,
        dcol: 5,
        row: 3,
        col: 5,
        label: "N",
        hint: "N/2",
    },
    KeyDef {
        drow: 4,
        dcol: 6,
        row: 3,
        col: 6,
        label: "M",
        hint: "M/3",
    },
    KeyDef {
        drow: 4,
        dcol: 7,
        row: 3,
        col: 7,
        label: "PGUP",
        hint: "PGUP",
    },
    KeyDef {
        drow: 4,
        dcol: 8,
        row: 6,
        col: 2,
        label: "UP",
        hint: "UP",
    },
    KeyDef {
        drow: 4,
        dcol: 9,
        row: 6,
        col: 6,
        label: "PGDN",
        hint: "PGDN",
    },
    // Bottom row.
    KeyDef {
        drow: 5,
        dcol: 0,
        row: 2,
        col: 0,
        label: "HELP",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 1,
        row: 2,
        col: 1,
        label: "SHIFT",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 2,
        row: 2,
        col: 2,
        label: "IME",
        hint: "",
    },
    KeyDef {
        drow: 5,
        dcol: 3,
        row: 2,
        col: 3,
        label: "ESC",
        hint: "ESC",
    },
    KeyDef {
        drow: 5,
        dcol: 4,
        row: 2,
        col: 4,
        label: "0",
        hint: "0",
    },
    KeyDef {
        drow: 5,
        dcol: 5,
        row: 2,
        col: 5,
        label: ".",
        hint: ".",
    },
    KeyDef {
        drow: 5,
        dcol: 6,
        row: 2,
        col: 6,
        label: "SPC",
        hint: "SPC",
    },
    KeyDef {
        drow: 5,
        dcol: 7,
        row: 2,
        col: 7,
        label: "LT",
        hint: "LT",
    },
    KeyDef {
        drow: 5,
        dcol: 8,
        row: 6,
        col: 3,
        label: "DN",
        hint: "DN",
    },
    KeyDef {
        drow: 5,
        dcol: 9,
        row: 6,
        col: 7,
        label: "RT",
        hint: "RT",
    },
];

/// Build an NC1020 key definition from its reference matrix ID.
const fn nc1020_key(
    drow: u8,
    dcol: u8,
    key_id: u8,
    label: &'static str,
    hint: &'static str,
) -> KeyDef {
    KeyDef {
        drow,
        dcol,
        row: key_id & 7,
        col: key_id >> 3,
        label,
        hint,
    }
}

/// NC1020 keypad using the matrix IDs from the reference implementation.
const LAYOUT_NC1020: &[KeyDef] = &[
    // Hotkeys: 英汉 名片 计算 行程 资料 时间 网络.
    nc1020_key(0, 0, 0x0B, "DICT", "F5"),
    nc1020_key(0, 1, 0x0C, "CARD", "F6"),
    nc1020_key(0, 2, 0x0D, "CALC", "F7"),
    nc1020_key(0, 3, 0x0A, "MEMO", "F8"),
    nc1020_key(0, 4, 0x09, "DATA", "F9"),
    nc1020_key(0, 5, 0x08, "TIME", "F10"),
    nc1020_key(0, 6, 0x0E, "NET", "F11"),
    // Side function keys and power.
    nc1020_key(1, 2, 0x10, "F1", "F1"),
    nc1020_key(1, 3, 0x11, "F2", "F2"),
    nc1020_key(1, 4, 0x12, "F3", "F3"),
    nc1020_key(1, 5, 0x13, "F4", "F4"),
    nc1020_key(1, 8, 0x0F, "ON", "DEL"),
    // QWERTY row.
    nc1020_key(2, 0, 0x20, "Q", "Q"),
    nc1020_key(2, 1, 0x21, "W", "W"),
    nc1020_key(2, 2, 0x22, "E", "E"),
    nc1020_key(2, 3, 0x23, "R", "R"),
    nc1020_key(2, 4, 0x24, "T/7", "T"),
    nc1020_key(2, 5, 0x25, "Y/8", "Y"),
    nc1020_key(2, 6, 0x26, "U/9", "U"),
    nc1020_key(2, 7, 0x27, "I", "I"),
    nc1020_key(2, 8, 0x18, "O", "O"),
    nc1020_key(2, 9, 0x1C, "P", "P"),
    nc1020_key(3, 0, 0x28, "A", "A"),
    nc1020_key(3, 1, 0x29, "S", "S"),
    nc1020_key(3, 2, 0x2A, "D", "D"),
    nc1020_key(3, 3, 0x2B, "F", "F"),
    nc1020_key(3, 4, 0x2C, "G/4", "G"),
    nc1020_key(3, 5, 0x2D, "H/5", "H"),
    nc1020_key(3, 6, 0x2E, "J/6", "J"),
    nc1020_key(3, 7, 0x2F, "K", "K"),
    nc1020_key(3, 8, 0x19, "L", "L"),
    nc1020_key(3, 9, 0x1D, "ENT", "ENT"),
    nc1020_key(4, 0, 0x30, "Z", "Z"),
    nc1020_key(4, 1, 0x31, "X", "X"),
    nc1020_key(4, 2, 0x32, "C", "C"),
    nc1020_key(4, 3, 0x33, "V", "V"),
    nc1020_key(4, 4, 0x34, "B/1", "B"),
    nc1020_key(4, 5, 0x35, "N/2", "N"),
    nc1020_key(4, 6, 0x36, "M/3", "M"),
    nc1020_key(4, 7, 0x37, "PGUP", "PGUP"),
    nc1020_key(4, 8, 0x1A, "UP", "UP"),
    nc1020_key(4, 9, 0x1E, "PGDN", "PGDN"),
    // Bottom row.
    nc1020_key(5, 0, 0x38, "HELP", ""),
    nc1020_key(5, 1, 0x39, "SHIFT", ""),
    nc1020_key(5, 2, 0x3A, "IME", ""),
    nc1020_key(5, 3, 0x3B, "ESC", "ESC"),
    nc1020_key(5, 4, 0x3C, "0", "0"),
    nc1020_key(5, 5, 0x3D, "DOT", "."),
    nc1020_key(5, 6, 0x3E, "SPC", "SPC"),
    nc1020_key(5, 7, 0x3F, "LT", "LT"),
    nc1020_key(5, 8, 0x1B, "DN", "DN"),
    nc1020_key(5, 9, 0x1F, "RT", "RT"),
];

/// Keypad layout for a model, used by frontends to draw the virtual
/// keypad and translate mouse clicks.
pub fn layout_for(model: MachineModel) -> &'static [KeyDef] {
    match model {
        MachineModel::Nc1020 => LAYOUT_NC1020,
        MachineModel::Nc2000 => LAYOUT_NC2000,
        MachineModel::Nc3000 => LAYOUT_NC3000,
        MachineModel::Pc1000 | MachineModel::Cc800 => LAYOUT_PC1000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nc1020_layout_uses_reference_matrix_ids() {
        let layout = layout_for(MachineModel::Nc1020);
        let expected = [
            ("DICT", 0x0B),
            ("CARD", 0x0C),
            ("CALC", 0x0D),
            ("MEMO", 0x0A),
            ("DATA", 0x09),
            ("TIME", 0x08),
            ("NET", 0x0E),
            ("Q", 0x20),
            ("A", 0x28),
            ("Z", 0x30),
            ("SPC", 0x3E),
        ];

        for (label, key_id) in expected {
            let key = layout.iter().find(|key| key.label == label).unwrap();
            assert_eq!(key_id_for(MachineModel::Nc1020, key.row, key.col), key_id);
        }
        assert_eq!(layout.len(), 52);
    }

    #[test]
    fn shared_host_keys_follow_each_models_layout() {
        let expectations = [
            (MachineModel::Nc1020, 0x1a),
            (MachineModel::Pc1000, 0x32),
            (MachineModel::Cc800, 0x32),
            (MachineModel::Nc2000, 0x13),
            (MachineModel::Nc3000, 0x13),
        ];

        for (model, expected) in expectations {
            assert_eq!(key_id_for_host_key(model, HostKey::Up), Some(expected));
        }
    }

    #[test]
    fn shared_host_keys_support_aliases_and_power_keys() {
        assert_eq!(
            key_id_for_host_key(MachineModel::Nc2000, HostKey::Letter('b')),
            key_id_for_host_key(MachineModel::Nc2000, HostKey::Digit(1))
        );
        for model in [
            MachineModel::Nc1020,
            MachineModel::Pc1000,
            MachineModel::Cc800,
            MachineModel::Nc2000,
            MachineModel::Nc3000,
        ] {
            assert_eq!(
                key_id_for_host_key(model, HostKey::Function(12)),
                key_id_for_host_key(model, HostKey::Delete)
            );
        }
    }

    #[test]
    fn frontend_input_keeps_same_source_aliases_pressed() {
        let mut input = FrontendInputState::default();
        let mut events = Vec::new();

        input.set_keyboard_key(HostKey::Letter('B'), true);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));
        input.set_keyboard_key(HostKey::Digit(1), true);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));
        input.set_keyboard_key(HostKey::Letter('B'), false);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));

        assert_eq!(events, vec![(0x26, true)]);

        input.set_keyboard_key(HostKey::Digit(1), false);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));
        assert_eq!(events, vec![(0x26, true), (0x26, false)]);
    }

    #[test]
    fn frontend_input_merges_keyboard_controller_and_pointer() {
        let mut input = FrontendInputState::default();
        let mut events = Vec::new();

        input.set_keyboard_key(HostKey::Up, true);
        input.set_controller_key(HostKey::Up, true);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));
        input.set_keyboard_key(HostKey::Up, false);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));
        input.set_pointer_key(Some(0x13));
        input.set_controller_key(HostKey::Up, false);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));

        assert_eq!(events, vec![(0x13, true)]);

        input.set_pointer_key(None);
        input.sync(MachineModel::Nc2000, |key, down| events.push((key, down)));
        assert_eq!(events, vec![(0x13, true), (0x13, false)]);
    }
}
