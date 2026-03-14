//! Fast Path — latency-critical pipeline.
//!
//! # HARD CONSTRAINTS (see .claude/rules/fast-path.md)
//! - **NO** async LLM API calls (OpenAI, Anthropic, Ollama, etc.)
//! - **NO** blocking HTTP / network I/O
//! - **NO** file I/O heavier than config reads
//! - All processing MUST complete in <250ms P50
//!
//! Violations will be caught by `cargo clippy` and blocked in CI.

use std::time::SystemTime;

use jarvis_core::{
    audio,
    audio_buffer::AudioRingBuffer,
    audio_processing, command_registry, commands, config, i18n, intent,
    ipc::{self, IpcEvent},
    recorder, scripts, slots, stt, voices, AssistantState, SOUND_DIR,
};

use crate::should_stop;

// VAD state machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VadState {
    WaitingForVoice,
    VoiceActive,
}

// Voice recognition for command after wake word
pub(crate) fn recognize_command(
    frame_buffer: &mut [i16],
    rt: &tokio::runtime::Runtime,
    frame_length: usize,
    sample_rate: usize,
    prefed_audio: bool,
) {
    let mut audio_buffer = AudioRingBuffer::new(2.0, frame_length, sample_rate);
    let mut vad_state = if prefed_audio {
        VadState::VoiceActive
    } else {
        VadState::WaitingForVoice
    };
    let mut silence_frames: u32 = 0;
    let mut start = SystemTime::now();
    let mut first_recognition = prefed_audio;

    // longer silence threshold for commands (user might pause to think)
    // 5 seconds
    let silence_threshold: u32 = ((5.0 * sample_rate as f32) / frame_length as f32) as u32;

    loop {
        if crate::should_stop() {
            return;
        }

        recorder::read_microphone(frame_buffer);
        let processed = audio_processing::process(frame_buffer);

        match vad_state {
            VadState::WaitingForVoice => {
                audio_buffer.push(frame_buffer);

                if processed.is_voice {
                    // flush buffer to STT
                    for buffered_frame in audio_buffer.drain_all() {
                        stt::recognize(&buffered_frame, false);
                    }
                    vad_state = VadState::VoiceActive;
                    silence_frames = 0;
                } else {
                    silence_frames += 1;

                    if silence_frames > silence_threshold {
                        info!("Long silence detected, returning to wake word mode.");
                        return;
                    }
                }
            }

            VadState::VoiceActive => {
                // feed to STT
                if let Some(mut recognized_voice) = stt::recognize(frame_buffer, false) {
                    info!("Recognized voice: {}", recognized_voice);

                    ipc::send(IpcEvent::SpeechRecognized {
                        text: recognized_voice.clone(),
                    });
                    ipc::send(IpcEvent::StateChanged {
                        state: AssistantState::Processing,
                    });

                    recognized_voice = recognized_voice.to_lowercase();

                    // check if wake word repeated (reactivate)
                    let wake_phrases = config::get_wake_phrases(&i18n::get_language());
                    let contains_wake = wake_phrases.iter().any(|wp| recognized_voice.contains(wp));

                    if contains_wake {
                        // strip the wake word
                        let mut remaining = recognized_voice.clone();
                        for wp in wake_phrases {
                            remaining = remaining.replace(wp, "");
                        }
                        let remaining = remaining.trim();

                        if remaining.is_empty() {
                            if first_recognition {
                                // leftover wake word from dual-feed, just discard it
                                info!("Discarding initial wake word from prefed audio");
                                first_recognition = false;
                                stt::reset_speech_recognizer();
                                voices::play_reply();
                                vad_state = VadState::WaitingForVoice;
                                silence_frames = 0;
                                start = SystemTime::now();
                                audio_buffer.clear();
                                continue;
                            }

                            // just wake word, no command - reactivate
                            info!("Wake word repeated during chaining, reactivating...");
                            voices::play_reply();
                            stt::reset_speech_recognizer();
                            ipc::send(IpcEvent::Listening);
                            ipc::send(IpcEvent::StateChanged {
                                state: AssistantState::Listening,
                            });

                            vad_state = VadState::WaitingForVoice;
                            silence_frames = 0;
                            start = SystemTime::now();
                            audio_buffer.clear();
                            continue;
                        } else {
                            // wake word + command in one phrase - execute the command part
                            info!("Wake word + command during chaining: '{}'", remaining);
                            recognized_voice = remaining.to_string();
                            // fall through to command execution below
                        }
                    }

                    first_recognition = false;

                    // filter activation phrases
                    // for tbr in config::ASSISTANT_PHRASES_TBR {
                    //     recognized_voice = recognized_voice.replace(tbr, "");
                    // }
                    for tbr in config::get_phrases_to_remove(&i18n::get_language()) {
                        recognized_voice = recognized_voice.replace(tbr, "");
                    }

                    recognized_voice = recognized_voice.trim().to_string();

                    if recognized_voice.len() < 5 {
                        debug!("Ignoring too short recognition: '{}'", recognized_voice);
                        continue;
                    }

                    if recognized_voice.is_empty() {
                        continue;
                    }

                    // execute command and check if we should chain
                    let should_chain = execute_command(&recognized_voice, rt);

                    if should_chain {
                        // chain: drain echo FIRST, then reset and continue listening.
                        // Without the drain the ok-sound echo triggers VAD → VoiceActive
                        // → Vosk accumulates garbage for up to 5 s (silence_threshold) → deafness.
                        info!("Chaining enabled, continuing to listen...");
                        debug!("[DEBUG] Resetting Audio Stream for Chaining");
                        drain_echo(frame_buffer, sample_rate, frame_length, 5);
                        stt::reset_speech_recognizer();
                        audio_processing::reset();
                        vad_state = VadState::WaitingForVoice;
                        silence_frames = 0;
                        start = SystemTime::now();
                        audio_buffer.clear();
                        ipc::send(IpcEvent::Listening);
                        ipc::send(IpcEvent::StateChanged {
                            state: AssistantState::Listening,
                        });
                        continue;
                    } else {
                        // no chain: pre-reset Vosk so buffer doesn't carry over
                        // into the echo drain + full reset that runs in main_loop.
                        info!("No chain, returning to wake word mode.");
                        stt::reset_speech_recognizer();
                        return;
                    }
                }

                // track silence
                if processed.is_voice {
                    silence_frames = 0;
                } else {
                    silence_frames += 1;

                    if silence_frames > silence_threshold {
                        info!("Long silence detected, returning to wake word mode.");
                        return;
                    }
                }
            }
        }

        // timeout
        if let Ok(elapsed) = start.elapsed() {
            if elapsed > config::CMS_WAIT_DELAY {
                info!("Command timeout, returning to wake word mode.");
                return;
            }
        }
    }
}

/// Drain microphone frames while Kira audio is playing, then add a dead zone
/// and a reverb tail drain. Call this after ANY audio playback + execute_command
/// to prevent speaker echo from reaching Vosk or the wake word detector.
///
/// - Reads and discards frames until audio::is_playing() returns false
/// - Adds 300 ms dead zone (flushes audio-card output buffer residue from PvRecorder)
/// - Adds 1 s reverb tail (clears remaining ring-buffer frames)
/// - Safety cap: never blocks longer than `max_secs` seconds total
pub(crate) fn drain_echo(
    frame_buffer: &mut [i16],
    sample_rate: usize,
    frame_length: usize,
    max_secs: u64,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(max_secs);
    let mut n: usize = 0;
    while !crate::should_stop() && audio::is_playing() && std::time::Instant::now() < deadline {
        recorder::read_microphone(frame_buffer);
        n += 1;
    }
    // 300 ms dead zone + 1 s reverb tail: discard audio-card / PvRecorder residue
    let extra = ((0.3 + 1.0) * sample_rate as f32 / frame_length as f32) as usize;
    for _ in 0..extra {
        recorder::read_microphone(frame_buffer);
    }
    debug!(
        "[EchoDrain] drained {} playback + {} tail = {} frames total",
        n,
        extra,
        n + extra
    );
}

pub(crate) fn process_text_command(text: &str, rt: &tokio::runtime::Runtime) {
    info!("Processing text command: {}", text);

    ipc::send(IpcEvent::SpeechRecognized {
        text: text.to_string(),
    });

    let mut filtered = text.to_lowercase();
    // for tbr in config::ASSISTANT_PHRASES_TBR {
    //     filtered = filtered.replace(tbr, "");
    // }
    for tbr in config::get_phrases_to_remove(&i18n::get_language()) {
        filtered = filtered.replace(tbr, "");
    }

    let filtered = filtered.trim();

    if filtered.is_empty() {
        ipc::send(IpcEvent::Idle);
        return;
    }

    // text commands never chain
    execute_command(filtered, rt);
}

// Execute command, returns true if chaining should continue
pub(crate) fn execute_command(text: &str, rt: &tokio::runtime::Runtime) -> bool {
    let commands_list = command_registry::get_snapshot();
    let commands_list = &*commands_list;

    let cmd_result = if let Some((intent_id, confidence)) = rt.block_on(intent::classify(text)) {
        info!(
            "Intent recognized: {} (confidence: {:.2})",
            intent_id, confidence
        );
        intent::get_command_by_intent(commands_list, &intent_id)
    } else {
        info!("Intent not recognized, trying levenshtein fallback...");
        commands::fetch_command(text, commands_list)
    };

    // Intent classification complete — now entering Responding phase.
    // Emitted once regardless of which branch handles the command (found, script, not-found).
    ipc::send(IpcEvent::StateChanged {
        state: AssistantState::Responding,
    });

    if let Some((cmd_path, cmd_config)) = cmd_result {
        info!("Command found: {:?}", cmd_path);

        // "script_ref" is a virtual command inserted by scripts::as_virtual_commands()
        // so the intent classifier trains on script phrases. Route to script engine.
        if cmd_config.cmd_type == "script_ref" {
            // Always load from disk — if the script was deleted via GUI it won't be found.
            if let Some(script) = scripts::load_script(&cmd_config.id) {
                info!("Script found via intent (live): {}", script.id);
                if !script.response_sound.is_empty() {
                    audio::play_sound(&SOUND_DIR.join(&script.response_sound));
                } else {
                    voices::play_random_from(script.get_sounds(&i18n::get_language()).as_slice());
                }
                match scripts::execute_script(&script) {
                    Ok(_) => {
                        ipc::send(IpcEvent::CommandExecuted {
                            id: script.id.clone(),
                            success: true,
                        });
                        ipc::send(IpcEvent::Idle);
                        return false;
                    }
                    Err(e) => {
                        error!("Script execution error: {}", e);
                        voices::play_error();
                        ipc::send(IpcEvent::CommandExecuted {
                            id: script.id.clone(),
                            success: false,
                        });
                        ipc::send(IpcEvent::Error { message: e });
                    }
                }
            } else {
                error!(
                    "Script '{}' not found on disk (deleted?), ignoring intent",
                    cmd_config.id
                );
                voices::play_not_found();
                ipc::send(IpcEvent::Error {
                    message: format!(
                        "Script '{}' was deleted — restart Jarvis to update voice triggers",
                        cmd_config.id
                    ),
                });
                ipc::send(IpcEvent::Idle);
            }
        } else {
            // Normal command execution
            let extracted_slots = if !cmd_config.slots.is_empty() {
                let s = slots::extract(text, &cmd_config.slots);
                if !s.is_empty() {
                    info!("Extracted slots: {:?}", s);
                }
                Some(s)
            } else {
                None
            };

            if !cmd_config.response_sound.is_empty() {
                audio::play_sound(&SOUND_DIR.join(&cmd_config.response_sound));
            } else {
                voices::play_random_from(cmd_config.get_sounds(&i18n::get_language()).as_slice());
            }

            match commands::execute_command(
                &cmd_path,
                &cmd_config,
                Some(&text),
                extracted_slots.as_ref(),
            ) {
                Ok(chain) => {
                    info!("Command executed successfully");
                    ipc::send(IpcEvent::CommandExecuted {
                        id: cmd_config.id.clone(),
                        success: true,
                    });
                    ipc::send(IpcEvent::Idle);
                    return chain;
                }
                Err(msg) => {
                    error!("Error executing command: {}", msg);
                    voices::play_error();
                    ipc::send(IpcEvent::CommandExecuted {
                        id: cmd_config.id.clone(),
                        success: false,
                    });
                    ipc::send(IpcEvent::Error {
                        message: msg.to_string(),
                    });
                }
            }
        }
    } else if let Some(script) = scripts::fetch_script_live(text) {
        info!(
            "[ROUTING] Script matched (live disk read): '{}' -> id='{}'",
            text, script.id
        );
        if !script.response_sound.is_empty() {
            audio::play_sound(&SOUND_DIR.join(&script.response_sound));
        } else {
            voices::play_random_from(script.get_sounds(&i18n::get_language()).as_slice());
        }
        match scripts::execute_script(&script) {
            Ok(_) => {
                ipc::send(IpcEvent::CommandExecuted {
                    id: script.id.clone(),
                    success: true,
                });
                ipc::send(IpcEvent::Idle);
                return false;
            }
            Err(e) => {
                error!("Script execution error: {}", e);
                voices::play_error();
                ipc::send(IpcEvent::Error { message: e });
            }
        }
    } else {
        // Intent not recognized AND fuzzy fallback failed — play "not found" line
        // ("Чего вы пытаетесь добиться, сэр?" — see resources/sound/jarvis-remaster/)
        info!("No command found for: '{}'", text);
        voices::play_not_found();
        // Reset speech recognizer immediately: prevents the not-found sound echo from
        // being fed into an already-full Vosk buffer on the next recognition pass.
        stt::reset_speech_recognizer();
        ipc::send(IpcEvent::Error {
            message: format!("Command not found: {}", text),
        });
    }

    ipc::send(IpcEvent::Idle);
    false // no chain on error or not found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: execute_command must return false (no chain) when no commands have
    /// been loaded via command_registry::load() (not yet populated from disk) — i.e. the assistant must return to Idle,
    /// never stay in Listening, regardless of what text was spoken.
    #[test]
    fn test_execute_command_returns_false_when_no_commands_loaded() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = execute_command("запусти ведьмака", &rt);
        assert!(
            !result,
            "execute_command must return false (no chain) when commands list is empty"
        );
    }
}
