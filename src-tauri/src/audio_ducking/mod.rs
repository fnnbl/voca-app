// Mute other applications' audio output while VOCA is recording, then
// restore the previous state when recording stops. Windows only for now —
// macOS has no clean per-app mute API and is tracked separately.
//
// Design note: we carry plain `(PID, session-instance-ID, session-ID)`
// triples across the mute→restore boundary, never COM objects. Each
// Win32 call initialises its own COM context and enumerates fresh,
// which keeps things thread-apartment-agnostic and avoids Send/Sync
// headaches.
//
// Two identifiers are tracked because Windows audio sessions have a
// lifecycle — Active → Inactive → Expired — and the right way to find
// a previously-muted session at restore time depends on what stage it
// is in:
//
// * `SessionInstanceIdentifier` (`IAudioSessionControl2`) uniquely
//   identifies a single session for its lifetime. Best match when the
//   exact session is still alive at restore time.
//
// * `SessionIdentifier` (also `IAudioSessionControl2`) is the
//   group-level identifier shared by every session for the same app.
//   Matching by group lets restore catch the case where the originally
//   muted session expired during a long recording and Chrome (or
//   another app) created a fresh session in the same group — fresh
//   sessions inherit Windows' persisted per-app mute state, so without
//   this fallback the new session comes up muted and stays muted.
//
// Inactive sessions are skipped at mute time entirely: there is no
// audio currently playing through them (e.g. paused video), so there
// is nothing to duck. Muting a paused session would only create the
// "stuck-muted" risk above without any benefit.

#[cfg(target_os = "windows")]
#[derive(Clone, Debug)]
pub(crate) struct MutedSession {
    pub pid: u32,
    /// Per-instance identifier — unique to one session, best match.
    pub instance_id: String,
    /// Group identifier — shared by every session for the same app.
    /// Used as fallback when the original session expired and was
    /// replaced by a fresh one in the same group.
    pub session_id: String,
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
        eMultimedia, eRender, AudioSessionStateActive, IAudioSessionControl2,
        IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume,
        MMDeviceEnumerator,
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
            let mut skipped_inactive = 0usize;
            let mut skipped_already_muted = 0usize;

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
                // Skip non-Active sessions: paused video, idle media
                // player etc. produce no audible output, so muting them
                // is pointless — and risks the "stuck muted" symptom
                // when the session later expires before we restore.
                let state = match control.GetState() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if state != AudioSessionStateActive {
                    skipped_inactive += 1;
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
                    skipped_already_muted += 1;
                    continue;
                }
                if volume.SetMute(true, std::ptr::null()).is_err() {
                    continue;
                }
                let instance_id = match control2.GetSessionInstanceIdentifier() {
                    Ok(p) => take_pwstr(p),
                    Err(_) => String::new(),
                };
                let session_id = match control2.GetSessionIdentifier() {
                    Ok(p) => take_pwstr(p),
                    Err(_) => String::new(),
                };
                muted.push(MutedSession {
                    pid,
                    instance_id,
                    session_id,
                });
            }

            log::info!(
                "audio ducking: muted {} active session(s) (skipped {} inactive, {} already muted)",
                muted.len(),
                skipped_inactive,
                skipped_already_muted
            );

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

            let tracked_instances: HashSet<&str> = sessions
                .iter()
                .filter(|s| !s.instance_id.is_empty())
                .map(|s| s.instance_id.as_str())
                .collect();
            let tracked_groups: HashSet<&str> = sessions
                .iter()
                .filter(|s| !s.session_id.is_empty())
                .map(|s| s.session_id.as_str())
                .collect();
            // PID-only fallback covers tracked entries from older
            // recordings where neither identifier was captured. New
            // entries always have at least the instance ID.
            let tracked_pids_legacy: HashSet<u32> = sessions
                .iter()
                .filter(|s| s.instance_id.is_empty() && s.session_id.is_empty())
                .map(|s| s.pid)
                .collect();

            let mut resolved_instances: HashSet<String> = HashSet::new();
            let mut resolved_groups: HashSet<String> = HashSet::new();
            let mut by_instance = 0usize;
            let mut by_group = 0usize;
            let mut by_pid_fallback = 0usize;

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
                let session_id = match control2.GetSessionIdentifier() {
                    Ok(p) => take_pwstr(p),
                    Err(_) => String::new(),
                };

                // Match priority: instance ID (precise) → group ID
                // (catches recreations within the same app) → PID
                // (legacy fallback only).
                let match_instance =
                    !instance_id.is_empty() && tracked_instances.contains(instance_id.as_str());
                let match_group = !match_instance
                    && !session_id.is_empty()
                    && tracked_groups.contains(session_id.as_str());
                let match_pid = !match_instance
                    && !match_group
                    && tracked_pids_legacy.contains(&pid);

                if !match_instance && !match_group && !match_pid {
                    continue;
                }

                let volume = match control.cast::<ISimpleAudioVolume>() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                // Unconditional un-mute — the per-session GetMute can
                // report unmuted while the persisted per-app entry
                // (which the Volume Mixer actually reflects) is still
                // muted. SetMute(false) is idempotent when already
                // unmuted.
                if volume.SetMute(false, std::ptr::null()).is_ok() {
                    if match_instance {
                        by_instance += 1;
                        resolved_instances.insert(instance_id.clone());
                    } else if match_group {
                        by_group += 1;
                    } else {
                        by_pid_fallback += 1;
                    }
                    if !session_id.is_empty() {
                        // Whether matched by instance, group, or PID,
                        // un-muting any session updates the persisted
                        // state for the whole group — record the group
                        // as resolved so other tracked sessions in the
                        // same group don't show up as unaccounted.
                        resolved_groups.insert(session_id);
                    }
                }
            }

            // A tracked session is considered handled if either its
            // own instance was un-muted or any session in its group
            // was un-muted (which clears the persisted per-app mute).
            let unaccounted = sessions
                .iter()
                .filter(|s| {
                    let inst_handled =
                        !s.instance_id.is_empty() && resolved_instances.contains(&s.instance_id);
                    let group_handled =
                        !s.session_id.is_empty() && resolved_groups.contains(&s.session_id);
                    !inst_handled && !group_handled
                })
                .count();

            if unaccounted > 0 {
                log::info!(
                    "audio ducking: {} of {} tracked session(s) not found on restore — \
                     likely all sessions in the group expired before recording ended",
                    unaccounted,
                    sessions.len()
                );
            }
            log::info!(
                "audio ducking: restored {} by instance, {} by group, {} via PID fallback",
                by_instance,
                by_group,
                by_pid_fallback
            );

            Ok(())
        }
    }

    /// Copy a COM-allocated wide string into a Rust String and free
    /// the underlying allocation. `GetSessionInstanceIdentifier` and
    /// `GetSessionIdentifier` both return LPWSTRs the caller is
    /// responsible for releasing via CoTaskMemFree.
    unsafe fn take_pwstr(p: windows::core::PWSTR) -> String {
        if p.0.is_null() {
            return String::new();
        }
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as _));
        s
    }
}
