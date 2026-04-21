use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn paste(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| format!("CLIPBOARD_ERROR: {e}"))?;

    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("PASTE_FAILED: {e}"))?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| format!("PASTE_FAILED: {e}"))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("PASTE_FAILED: {e}"))?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| format!("PASTE_FAILED: {e}"))?;

    Ok(())
}
