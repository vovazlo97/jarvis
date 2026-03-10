use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// Unix timestamp (ms) when the last scheduled audio playback is expected to end.
// Set by play_sound(); read by is_playing().
static PLAYBACK_ENDS_AT_MS: AtomicU64 = AtomicU64::new(0);

// use kira::{
//     manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
//     sound::static_sound::{StaticSoundData, StaticSoundSettings},
// };

use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    sound::static_sound::StaticSoundData,
};

static MANAGER: OnceCell<Mutex<AudioManager>> = OnceCell::new();

pub fn init() -> Result<(), ()> {
    if MANAGER.get().is_some() {
        return Ok(());
    }  // already initialized

    // Create an audio manager. This plays sounds and manages resources.
    match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
        Ok(manager) => {
            // store
            MANAGER.set(Mutex::new(manager)).ok();

            // success
            Ok(())
        }
        Err(msg) => {
            error!("Failed to initialize audio stream.\nError details: {}", msg);

            // failed
            Err(())
        }
    }
}

/// Returns true if Kira audio playback is currently in progress.
/// Used to gate microphone reading after command execution to prevent
/// speaker echo from corrupting Vosk wake word detection.
pub fn is_playing() -> bool {
    let ends_at = PLAYBACK_ENDS_AT_MS.load(Ordering::SeqCst);
    if ends_at == 0 {
        return false;
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    now_ms < ends_at
}

// @TODO. Cache sounds in memory? With a pool of a certain size, for instance.
pub fn play_sound(filename: &PathBuf) {
    // load the file
    match StaticSoundData::from_file(filename) {
        Ok(sound_data) => {
            // Track when this audio will finish so the voice loop can gate on it.
            let duration_ms = sound_data.duration().as_millis() as u64;
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            PLAYBACK_ENDS_AT_MS.store(now_ms + duration_ms, Ordering::SeqCst);

            // play it (non-blocking)
            if let Some(manager) = MANAGER.get() {
                if let Ok(mut audio_manager) = manager.lock() {
                    if let Err(e) = audio_manager.play(sound_data) {
                        warn!("Failed to play sound: {}", e);
                    }
                }
            } else {
                warn!("Audio manager not initialized");
            }
        }
        Err(err) => {
            warn!("Cannot find sound file: {} (err: {})", filename.display(), err);
        }
    }
}