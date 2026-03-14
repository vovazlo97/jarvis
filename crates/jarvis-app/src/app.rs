use std::sync::mpsc::Receiver;

use jarvis_core::{
    audio_buffer::AudioRingBuffer,
    audio_processing,
    ipc::{self, IpcEvent},
    listener, recorder, stt, voices, AssistantState,
};

use crate::{
    fast_path::{drain_echo, process_text_command, recognize_command, VadState},
    should_stop,
};

pub fn start(text_cmd_rx: Receiver<String>, rt: &tokio::runtime::Runtime) -> Result<(), ()> {
    main_loop(text_cmd_rx, rt)
}

fn main_loop(text_cmd_rx: Receiver<String>, rt: &tokio::runtime::Runtime) -> Result<(), ()> {
    let frame_length: usize = 512;
    let sample_rate: usize = 16000;
    let mut frame_buffer: Vec<i16> = vec![0; frame_length];

    // ring buffer: keeps last 5 seconds of audio (pre-roll)
    let mut audio_buffer = AudioRingBuffer::new(5.0, frame_length, sample_rate);

    // VAD state
    let mut vad_state = VadState::WaitingForVoice;
    let mut silence_frames: u32 = 0;

    // how many frames of silence before we consider speech ended
    // 1.5 seconds = 1.5 * (16000 / 512) ≈ 47 frames
    let silence_threshold: u32 = ((1.5 * sample_rate as f32) / frame_length as f32) as u32;

    voices::play_greet();
    // Give Kira's non-blocking audio time to actually start playing before the
    // microphone opens (PvRecorder can steal the audio device on some Windows configs)
    std::thread::sleep(std::time::Duration::from_millis(800));

    match recorder::start_recording() {
        Ok(_) => info!(
            "Recording started. Microphone: {}",
            recorder::get_audio_device_name(recorder::get_selected_microphone_index())
        ),
        Err(_) => {
            error!("Cannot start recording.");
            return Err(());
        }
    }

    ipc::send(IpcEvent::Idle);
    ipc::send(IpcEvent::StateChanged {
        state: AssistantState::Idle,
    });

    // ### WAKE WORD DETECTION LOOP
    let mut audio_chunk_count: u64 = 0;
    // log every N chunks: 16000/512 * 5 ≈ 156 chunks = ~5 seconds
    const AUDIO_LOG_INTERVAL: u64 = 156;

    'wake_word: loop {
        if should_stop() {
            info!("Stop signal received, shutting down...");
            voices::play_goodbye();
            ipc::send(IpcEvent::Stopping);
            break;
        }

        if let Ok(text) = text_cmd_rx.try_recv() {
            process_text_command(&text, rt);
            continue 'wake_word;
        }

        recorder::read_microphone(&mut frame_buffer);
        audio_chunk_count += 1;
        if audio_chunk_count.is_multiple_of(AUDIO_LOG_INTERVAL) {
            debug!(
                "[Audio] chunk #{} received ({} samples), vad_state={:?}",
                audio_chunk_count,
                frame_buffer.len(),
                vad_state
            );
        }
        let processed = audio_processing::process(&frame_buffer);

        match vad_state {
            VadState::WaitingForVoice => {
                // always buffer audio
                audio_buffer.push(&frame_buffer);

                if processed.is_voice {
                    // voice started! flush buffer to Vosk
                    info!(
                        "VAD: Voice started, flushing {} buffered frames",
                        audio_buffer.len()
                    );

                    for buffered_frame in audio_buffer.drain_all() {
                        listener::data_callback(&buffered_frame);
                    }

                    vad_state = VadState::VoiceActive;
                    silence_frames = 0;
                }
            }

            VadState::VoiceActive => {
                // dual-feed: speech recognizer gets frames in parallel with wake word detector
                let _ = stt::recognize(&frame_buffer, false);

                // Wake-word detection is ONLY active here, in the outer 'wake_word loop
                // (AssistantState::Idle). Once recognize_command() is called the outer
                // loop is suspended entirely — the inner loop never calls data_callback(),
                // so the detector cannot fire during command execution or chaining.
                if let Some(_keyword_index) = listener::data_callback(&frame_buffer) {
                    // WAKE WORD DETECTED!
                    info!("Wake word activated!");
                    ipc::send(IpcEvent::WakeWordDetected);
                    ipc::send(IpcEvent::StateChanged {
                        state: AssistantState::Activated,
                    });

                    stt::reset_wake_recognizer();
                    // Reset speech recognizer so it starts fresh for the upcoming command
                    // (it accumulated wake-word audio that can't be recognized in free-vocab mode)
                    stt::reset_speech_recognizer();
                    audio_processing::reset();

                    // Signal to the user that Jarvis is now listening for a command
                    voices::play_reply();

                    ipc::send(IpcEvent::Listening);
                    ipc::send(IpcEvent::StateChanged {
                        state: AssistantState::Listening,
                    });
                    recognize_command(&mut frame_buffer, rt, frame_length, sample_rate, false);

                    // Drain speaker echo before re-entering wake word detection.
                    // (Kira is non-blocking; response sounds keep playing after return.)
                    drain_echo(&mut frame_buffer, sample_rate, frame_length, 8);

                    // reset state after command
                    vad_state = VadState::WaitingForVoice;
                    silence_frames = 0;
                    audio_buffer.clear();
                    stt::reset_wake_recognizer();
                    stt::reset_speech_recognizer(); // NOW reset, after command is done
                    audio_processing::reset();
                    ipc::send(IpcEvent::Idle);
                    ipc::send(IpcEvent::StateChanged {
                        state: AssistantState::Idle,
                    });

                    continue 'wake_word;
                }

                // track silence
                if processed.is_voice {
                    silence_frames = 0;
                } else {
                    silence_frames += 1;

                    if silence_frames > silence_threshold {
                        debug!("VAD: Silence timeout, returning to wait state");
                        vad_state = VadState::WaitingForVoice;
                        silence_frames = 0;
                        stt::reset_wake_recognizer();
                        stt::reset_speech_recognizer(); // reset since we were dual-feeding
                    }
                }
            }
        }
    }

    recorder::stop_recording().ok();
    ipc::send(IpcEvent::Stopping);

    Ok(())
}

pub fn close(code: i32) {
    info!("Closing application.");
    voices::play_goodbye();
    ipc::send(IpcEvent::Stopping);
    std::process::exit(code);
}
