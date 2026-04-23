// Mute other applications' audio output while VOCA is recording, then
// restore the previous state when recording stops. Windows only for now —
// macOS has no clean per-app mute API and is tracked separately.
//
// Design note: we only carry plain PIDs across the mute→restore boundary,
// never COM objects. Each Win32 call initialises its own COM context and
// enumerates fresh, which keeps things thread-apartment-agnostic and
// avoids Send/Sync headaches.

pub struct DuckingGuard {
    #[cfg(target_os = "windows")]
    muted_pids: Vec<u32>,
    #[cfg(not(target_os = "windows"))]
    _phantom: (),
}

impl DuckingGuard {
    fn empty() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            muted_pids: Vec::new(),
            #[cfg(not(target_os = "windows"))]
            _phantom: (),
        }
    }
}

pub fn mute_others() -> DuckingGuard {
    #[cfg(target_os = "windows")]
    {
        match windows_impl::mute() {
            Ok(pids) => DuckingGuard { muted_pids: pids },
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
        if guard.muted_pids.is_empty() {
            return;
        }
        if let Err(e) = windows_impl::restore(&guard.muted_pids) {
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
    use windows::core::Interface;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDevice,
        IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    pub fn mute() -> Result<Vec<u32>, String> {
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
            let mut muted: Vec<u32> = Vec::new();

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
                if volume.SetMute(true, std::ptr::null()).is_ok() {
                    muted.push(pid);
                }
            }

            Ok(muted)
        }
    }

    pub fn restore(pids: &[u32]) -> Result<(), String> {
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
                if !pids.contains(&pid) {
                    continue;
                }
                let volume = match control.cast::<ISimpleAudioVolume>() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let _ = volume.SetMute(false, std::ptr::null());
            }

            Ok(())
        }
    }
}
