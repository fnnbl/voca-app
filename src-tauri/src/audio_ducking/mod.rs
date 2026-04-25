// Mute other applications' audio output while VOCA is recording, then
// restore the previous state when recording stops. Windows only for now —
// macOS has no clean per-app mute API and is tracked separately.
//
// Design note: we carry plain `(PID, session-instance-ID)` pairs across
// the mute→restore boundary, never COM objects. Each Win32 call
// initialises its own COM context and enumerates fresh, which keeps
// things thread-apartment-agnostic and avoids Send/Sync headaches.
//
// The session instance identifier (returned by
// `IAudioSessionControl2::GetSessionInstanceIdentifier`) uniquely
// identifies a single audio session for the lifetime of that session,
// and stays stable even when the session toggles between active and
// inactive (e.g. a paused video). Matching restore against the instance
// ID first — with PID as a fallback — fixes a class of bugs where a
// session muted during recording would get left muted because
// PID-only matching failed to re-locate it (multi-renderer Chrome
// is the canonical example: one Chrome.exe, many audio sessions, and
// PID alone can't distinguish them).

#[cfg(target_os = "windows")]
#[derive(Clone, Debug)]
pub(crate) struct MutedSession {
    pub pid: u32,
    pub instance_id: String,
}

pub struct DuckingGuard {
    #[cfg(target_os = "windows")]
    muted: Vec<MutedSession>,
    #[cfg(not(target_os = "windows"))]
    _phantom: (),
}

impl DuckingGuard {
    fn empty() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            muted: Vec::new(),
            #[cfg(not(target_os = "windows"))]
            _phantom: (),
        }
    }
}

pub fn mute_others() -> DuckingGuard {
    #[cfg(target_os = "windows")]
    {
        match windows_impl::mute() {
            Ok(muted) => DuckingGuard { muted },
            Err(e) => {
                log::warn!("audio ducking: mute failed: {e}");
                DuckingGuard::empty()
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        DuckingGuard::empty()
    }
}

pub fn restore(guard: DuckingGuard) {
    #[cfg(target_os = "windows")]
    {
        if guard.muted.is_empty() {
            return;
        }
        if let Err(e) = windows_impl::restore(&guard.muted) {
            log::warn!("audio ducking: restore failed: {e}");
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        drop(guard);
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::MutedSession;
    use std::collections::HashSet;
    use windows::core::Interface;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDevice,
        IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    pub fn mute() -> Result<Vec<MutedSession>, String> {
        unsafe {
            // CoInitializeEx is idempotent per-thread; RPC_E_CHANGED_MODE means
            // another apartment is already set, which is fine for our usage
            // since every call re-creates its own COM objects.
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("IMMDeviceEnumerator: {e}"))?;

            let device: IMMDevice = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|e| format!("GetDefaultAudioEndpoint: {e}"))?;

            let manager: IAudioSessionManager2 = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioSessionManager2: {e}"))?;

            let session_enum = manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {e}"))?;

            let count = session_enum
                .GetCount()
                .map_err(|e| format!("GetCount: {e}"))?;

            let our_pid = std::process::id();
            let mut muted: Vec<MutedSession> = Vec::new();

            for i in 0..count {
                let control = match session_enum.GetSession(i) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let control2 = match control.cast::<IAudioSessionControl2>() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let pid = match control2.GetProcessId() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                if pid == 0 || pid == our_pid {
                    continue;
                }
                let volume = match control.cast::<ISimpleAudioVolume>() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                // Already-muted sessions aren't ours to touch — leave them
                // alone so we don't "un-mute" a session the user muted
                // deliberately.
                let was_muted = volume.GetMute().unwrap_or_default();
                if was_muted.as_bool() {
                    continue;
                }
                if volume.SetMute(true, std::ptr::null()).is_err() {
                    continue;
                }
                let instance_id = match control2.GetSessionInstanceIdentifier() {
                    Ok(p) => take_pwstr(p),
                    Err(_) => String::new(),
                };
                muted.push(MutedSession { pid, instance_id });
            }

            Ok(muted)
        }
    }

    pub fn restore(sessions: &[MutedSession]) -> Result<(), String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("IMMDeviceEnumerator: {e}"))?;

            let device: IMMDevice = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|e| format!("GetDefaultAudioEndpoint: {e}"))?;

            let manager: IAudioSessionManager2 = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioSessionManager2: {e}"))?;

            let session_enum = manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {e}"))?;

            let count = session_enum
                .GetCount()
                .map_err(|e| format!("GetCount: {e}"))?;

            let tracked_ids: HashSet<&str> = sessions
                .iter()
                .filter(|s| !s.instance_id.is_empty())
                .map(|s| s.instance_id.as_str())
                .collect();
            let tracked_pids_without_id: HashSet<u32> = sessions
                .iter()
                .filter(|s| s.instance_id.is_empty())
                .map(|s| s.pid)
                .collect();

            let mut restored_ids: HashSet<String> = HashSet::new();
            let mut restored_by_id = 0usize;
            let mut restored_by_pid_fallback = 0usize;

            for i in 0..count {
                let control = match session_enum.GetSession(i) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let control2 = match control.cast::<IAudioSessionControl2>() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let pid = match control2.GetProcessId() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let instance_id = match control2.GetSessionInstanceIdentifier() {
                    Ok(p) => take_pwstr(p),
                    Err(_) => String::new(),
                };

                // Match policy: instance ID first (uniquely identifies
                // a single session, survives multi-session-per-PID cases
                // like Chrome's per-renderer audio), then PID fallback
                // for tracked entries that didn't capture an instance ID.
                let matched_by_id = !instance_id.is_empty()
                    && tracked_ids.contains(instance_id.as_str());
                let matched_by_pid =
                    !matched_by_id && tracked_pids_without_id.contains(&pid);

                if !matched_by_id && !matched_by_pid {
                    continue;
                }

                let volume = match control.cast::<ISimpleAudioVolume>() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                // Unconditional un-mute: don't trust the current GetMute
                // state — the session may report unmuted while the
                // mixer entry is still stuck muted (the symptom that
                // motivated this rewrite). SetMute(false) is idempotent
                // when the session is genuinely already unmuted.
                if volume.SetMute(false, std::ptr::null()).is_ok() {
                    if matched_by_id {
                        restored_by_id += 1;
                        restored_ids.insert(instance_id);
                    } else {
                        restored_by_pid_fallback += 1;
                    }
                }
            }

            // Sessions we tracked but didn't find on restore. Most likely
            // the session was destroyed before recording ended (renderer
            // process gone, tab closed, audio engine restarted) — in
            // which case the mute went with it and there's nothing for
            // us to un-mute. Log if it happens so a future regression
            // is visible.
            let unaccounted_ids: Vec<&str> = sessions
                .iter()
                .filter(|s| !s.instance_id.is_empty() && !restored_ids.contains(&s.instance_id))
                .map(|s| s.instance_id.as_str())
                .collect();
            if !unaccounted_ids.is_empty() {
                log::info!(
                    "audio ducking: {} tracked session(s) not found on restore",
                    unaccounted_ids.len()
                );
            }
            log::debug!(
                "audio ducking: restored {} by instance ID, {} via PID fallback",
                restored_by_id,
                restored_by_pid_fallback
            );

            Ok(())
        }
    }

    /// Copy a COM-allocated wide string into a Rust String and free the
    /// underlying allocation. `GetSessionInstanceIdentifier` returns an
    /// LPWSTR the caller is responsible for releasing via CoTaskMemFree.
    unsafe fn take_pwstr(p: windows::core::PWSTR) -> String {
        if p.0.is_null() {
            return String::new();
        }
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as _));
        s
    }
}
