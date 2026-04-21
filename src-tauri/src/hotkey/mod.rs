use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    thread,
};

use rdev::{listen, Event, EventType, Key as RKey};
use tauri::AppHandle;

// Abstract modifier/key token — maps both Left and Right variants to the same value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Token {
    Ctrl,
    Alt,
    Shift,
    Super,
    Named(&'static str),
    Char(char),
}

fn parse_shortcut(s: &str) -> HashSet<Token> {
    s.split('+').filter_map(|part| str_to_token(part.trim())).collect()
}

fn str_to_token(s: &str) -> Option<Token> {
    match s {
        "Ctrl" | "Control" => Some(Token::Ctrl),
        "Alt" => Some(Token::Alt),
        "Shift" => Some(Token::Shift),
        "Super" | "Win" | "Meta" | "Cmd" => Some(Token::Super),
        "Space" => Some(Token::Named("Space")),
        "Return" | "Enter" => Some(Token::Named("Return")),
        "Tab" => Some(Token::Named("Tab")),
        "Backspace" | "BackSpace" => Some(Token::Named("Backspace")),
        "Escape" | "Esc" => Some(Token::Named("Escape")),
        "Delete" | "Del" => Some(Token::Named("Delete")),
        s if s.len() == 1 => Some(Token::Char(s.chars().next()?.to_ascii_uppercase())),
        s if s.starts_with('F') => {
            let _n: u8 = s[1..].parse().ok()?;
            Some(Token::Named(match s {
                "F1" => "F1", "F2" => "F2", "F3" => "F3", "F4" => "F4",
                "F5" => "F5", "F6" => "F6", "F7" => "F7", "F8" => "F8",
                "F9" => "F9", "F10" => "F10", "F11" => "F11", "F12" => "F12",
                _ => return None,
            }))
        }
        _ => None,
    }
}

fn rkey_to_token(k: &RKey) -> Option<Token> {
    use RKey::*;
    match k {
        ControlLeft | ControlRight => Some(Token::Ctrl),
        Alt | AltGr => Some(Token::Alt),
        ShiftLeft | ShiftRight => Some(Token::Shift),
        MetaLeft | MetaRight => Some(Token::Super),
        Space => Some(Token::Named("Space")),
        Return => Some(Token::Named("Return")),
        Tab => Some(Token::Named("Tab")),
        Backspace => Some(Token::Named("Backspace")),
        Escape => Some(Token::Named("Escape")),
        Delete => Some(Token::Named("Delete")),
        KeyA => Some(Token::Char('A')),
        KeyB => Some(Token::Char('B')),
        KeyC => Some(Token::Char('C')),
        KeyD => Some(Token::Char('D')),
        KeyE => Some(Token::Char('E')),
        KeyF => Some(Token::Char('F')),
        KeyG => Some(Token::Char('G')),
        KeyH => Some(Token::Char('H')),
        KeyI => Some(Token::Char('I')),
        KeyJ => Some(Token::Char('J')),
        KeyK => Some(Token::Char('K')),
        KeyL => Some(Token::Char('L')),
        KeyM => Some(Token::Char('M')),
        KeyN => Some(Token::Char('N')),
        KeyO => Some(Token::Char('O')),
        KeyP => Some(Token::Char('P')),
        KeyQ => Some(Token::Char('Q')),
        KeyR => Some(Token::Char('R')),
        KeyS => Some(Token::Char('S')),
        KeyT => Some(Token::Char('T')),
        KeyU => Some(Token::Char('U')),
        KeyV => Some(Token::Char('V')),
        KeyW => Some(Token::Char('W')),
        KeyX => Some(Token::Char('X')),
        KeyY => Some(Token::Char('Y')),
        KeyZ => Some(Token::Char('Z')),
        Num0 => Some(Token::Char('0')),
        Num1 => Some(Token::Char('1')),
        Num2 => Some(Token::Char('2')),
        Num3 => Some(Token::Char('3')),
        Num4 => Some(Token::Char('4')),
        Num5 => Some(Token::Char('5')),
        Num6 => Some(Token::Char('6')),
        Num7 => Some(Token::Char('7')),
        Num8 => Some(Token::Char('8')),
        Num9 => Some(Token::Char('9')),
        F1 => Some(Token::Named("F1")),
        F2 => Some(Token::Named("F2")),
        F3 => Some(Token::Named("F3")),
        F4 => Some(Token::Named("F4")),
        F5 => Some(Token::Named("F5")),
        F6 => Some(Token::Named("F6")),
        F7 => Some(Token::Named("F7")),
        F8 => Some(Token::Named("F8")),
        F9 => Some(Token::Named("F9")),
        F10 => Some(Token::Named("F10")),
        F11 => Some(Token::Named("F11")),
        F12 => Some(Token::Named("F12")),
        _ => None,
    }
}

pub struct HotkeyState {
    target: Arc<Mutex<HashSet<Token>>>,
    pressed: Arc<Mutex<HashSet<Token>>>,
    active: Arc<Mutex<bool>>,
}

impl HotkeyState {
    pub fn new() -> Self {
        Self {
            target: Arc::new(Mutex::new(HashSet::new())),
            pressed: Arc::new(Mutex::new(HashSet::new())),
            active: Arc::new(Mutex::new(false)),
        }
    }

    pub fn set_shortcut(&self, shortcut: &str) {
        let parsed = parse_shortcut(shortcut);
        eprintln!("[VOCA hotkey] set_shortcut('{shortcut}') -> {} tokens", parsed.len());
        *self.target.lock().unwrap() = parsed;
        self.pressed.lock().unwrap().clear();
        *self.active.lock().unwrap() = false;
    }
}

pub fn start(state: Arc<HotkeyState>, app: AppHandle) {
    thread::spawn(move || {
        eprintln!("[VOCA hotkey] listener thread starting");
        let target = state.target.clone();
        let pressed = state.pressed.clone();
        let active = state.active.clone();

        let result = listen(move |event: Event| {
            let (token, is_press) = match &event.event_type {
                EventType::KeyPress(k) => match rkey_to_token(k) {
                    Some(t) => (t, true),
                    None => return,
                },
                EventType::KeyRelease(k) => match rkey_to_token(k) {
                    Some(t) => (t, false),
                    None => return,
                },
                _ => return,
            };

            let target_keys = target.lock().unwrap().clone();
            if target_keys.is_empty() {
                return;
            }

            {
                let mut p = pressed.lock().unwrap();
                if is_press {
                    p.insert(token);
                } else {
                    p.remove(&token);
                }
            }

            let pressed_keys = pressed.lock().unwrap().clone();
            let is_active = target_keys.iter().all(|t| pressed_keys.contains(t));

            let mut was_active = active.lock().unwrap();
            if is_active && !*was_active {
                *was_active = true;
                drop(was_active);
                eprintln!("[VOCA hotkey] -> on_press");
                crate::shortcut::on_press(&app);
            } else if !is_active && *was_active {
                *was_active = false;
                drop(was_active);
                eprintln!("[VOCA hotkey] -> on_release");
                crate::shortcut::on_release(&app);
            }
        });

        if let Err(e) = result {
            eprintln!("[VOCA hotkey] listener EXITED with error: {e:?}");
            eprintln!("[VOCA hotkey] On macOS: grant Accessibility permission to this app.");
        } else {
            eprintln!("[VOCA hotkey] listener exited normally");
        }
    });
}
