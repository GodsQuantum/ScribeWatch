//! Native Rust microphone capture and dictation transport.
//! Uses Handy's MIT-licensed handy-recorder; never accumulates a long
//! recording in memory, starts no extra engine, and imposes no time limit.
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use handy_recorder::{AudioChunk, Recorder, RecorderConfig, Sink};
use hound::{SampleFormat, WavSpec, WavWriter};
use serde::Deserialize;
use std::{
    fs::{self, File, OpenOptions},
    io::BufWriter,
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

struct WavSink {
    writer: Option<WavWriter<BufWriter<File>>>,
    error: Option<String>,
    frames: u64,
}

impl WavSink {
    fn create(path: &Path) -> Result<Self, String> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path).map_err(|e| e.to_string())?;
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let writer = WavWriter::new(BufWriter::new(file), spec).map_err(|e| e.to_string())?;
        Ok(Self {
            writer: Some(writer),
            error: None,
            frames: 0,
        })
    }

    fn finish(mut self) -> Result<u64, String> {
        if let Some(writer) = self.writer.take() {
            writer.finalize().map_err(|e| e.to_string())?;
        }
        if let Some(error) = self.error.take() {
            return Err(format!("Microphone write failure: {error}"));
        }
        if self.frames == 0 {
            return Err("Microphone captured no audio".into());
        }
        Ok(self.frames)
    }
}

impl Sink for WavSink {
    fn process_chunk(&mut self, chunk: AudioChunk<'_>) {
        if self.error.is_some() {
            return;
        }
        if chunk.sample_rate != 16_000 || chunk.channels != 1 {
            self.error = Some(format!(
                "Unexpected microphone stream: {} Hz {} channel(s)",
                chunk.sample_rate, chunk.channels
            ));
            return;
        }
        let Some(writer) = self.writer.as_mut() else {
            return;
        };
        for sample in chunk.samples.iter().take(chunk.valid_frames) {
            let pcm = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            if let Err(error) = writer.write_sample(pcm) {
                self.error = Some(error.to_string());
                return;
            }
        }
        self.frames += chunk.valid_frames as u64;
    }
}

pub struct NativeDictation {
    recorder: Recorder<WavSink>,
    path: PathBuf,
}

impl NativeDictation {
    pub fn start(base: &Path) -> Result<Self, String> {
        let directory = base.join("data/dictation-recovery");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        // Remove only old temporary failed dictations. A successful ASR
        // already deletes its WAV. No timer, polling job or extra service.
        let expiry = std::time::SystemTime::now()
            .checked_sub(Duration::from_secs(24 * 3600));
        if let Some(expiry) = expiry {
            if let Ok(entries) = fs::read_dir(&directory) {
                for entry in entries.flatten() {
                    if entry.path().extension().is_some_and(|ext| ext == "wav")
                        && entry.metadata().ok()
                            .and_then(|m| m.modified().ok())
                            .is_some_and(|time| time < expiry)
                    {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
        let path = directory.join(format!("{}.wav", Uuid::new_v4()));
        let sink = WavSink::create(&path)?;
        let recorder = Recorder::open_with_failure_handler(RecorderConfig::speech(), |e| {
            eprintln!("ScribeWatch microphone device error: {e}");
        })
        .map_err(|e| {
            let _ = fs::remove_file(&path);
            e.to_string()
        })?;
        if let Err(e) = recorder.start(sink) {
            let _ = fs::remove_file(&path);
            return Err(e.to_string());
        }
        Ok(Self { recorder, path })
    }

    pub fn stop(self) -> Result<PathBuf, String> {
        let completed = self.recorder.stop().map_err(|e| e.to_string())?;
        let end_reason = completed.end_reason.clone();
        let dropped_frames = completed.dropped_frames;
        let frames = completed.sink.finish()?;
        if dropped_frames > 0 || !matches!(end_reason, handy_recorder::EndReason::StopCalled) {
            return Err(format!(
                "Incomplete microphone recording: {frames} samples, {dropped_frames} dropped frames, {end_reason:?}. Recovery audio: {}",
                self.path.display()
            ));
        }
        Ok(self.path)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DictationSettings {
    #[serde(default)]
    structure_profile_id: Option<String>,
}
#[derive(Deserialize)]
struct Transcript {
    text: String,
}
#[derive(Deserialize)]
struct Structured {
    markdown: String,
}

fn preserve_transcript_on_structure_failure(
    original: String,
    structured: Result<String, String>,
) -> String {
    match structured {
        Ok(markdown) if !markdown.trim().is_empty() => markdown,
        Ok(_) => original,
        Err(error) => {
            eprintln!("ScribeWatch optional LLM unavailable, preserving transcript: {error}");
            original
        }
    }
}

/// Called only after native microphone has stopped. WAV streams from disk
/// to localhost; the existing backend chooses the saved local/remote model.
pub fn transcribe_to_clipboard(port: u16, file: PathBuf) -> Result<String, String> {
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let settings: DictationSettings = client
        .get(format!("{base}/api/v1/dictation/settings"))
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    let part = reqwest::blocking::multipart::Part::file(&file)
        .map_err(|e| e.to_string())?
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;
    let form = reqwest::blocking::multipart::Form::new().part("file", part);
    let transcript: Transcript = client
        .post(format!("{base}/api/v1/dictation/transcribe"))
        .multipart(form)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;
    let text = if let Some(profile_id) = settings.structure_profile_id.filter(|id| !id.is_empty()) {
        // The LLM is optional. A cloud/local LLM failure must never discard
        // a successfully recognized transcript or delay the clipboard forever.
        let structured = client
            .post(format!("{base}/api/v1/structure/existing"))
            .json(&serde_json::json!({"profileId": profile_id, "transcript": &transcript.text}))
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json::<Structured>())
            .map(|r| r.markdown)
            .map_err(|e| e.to_string());
        preserve_transcript_on_structure_failure(transcript.text, structured)
    } else {
        transcript.text
    };
    if text.trim().is_empty() {
        return Err("ASR returned empty text".into());
    }
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    // Keep the transcript in the OS clipboard if key injection is unavailable
    // (e.g. KDE Wayland awaiting portal permission). Never pretend it pasted.
    let paste_result = paste_to_focused_application();
    // The transcript is now recoverable from the clipboard. Do not retain
    // confidential recordings after a successful transcription.
    if let Err(error) = fs::remove_file(&file) {
        eprintln!("ScribeWatch dictation temporary audio cleanup: {error}");
    }
    match paste_result {
        Ok(()) => Ok("Dictée transcrite et insérée dans l’application active".into()),
        Err(error) => {
            eprintln!("ScribeWatch paste unavailable: {error}");
            Ok("Dictée copiée dans le presse-papiers ; collage manuel nécessaire".into())
        }
    }
}

/// Simulates the platform's paste shortcut via Rust. The target window
/// remains focused because the tray never opens a dictation window.
fn paste_to_focused_application() -> Result<(), String> {
    let mut keys = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    keys.key(modifier, Direction::Press)
        .map_err(|e| e.to_string())?;
    let click = keys.key(Key::Unicode('v'), Direction::Click);
    let release = keys.key(modifier, Direction::Release);
    click.map_err(|e| e.to_string())?;
    release.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_llm_failure_preserves_exact_asr_transcript() {
        let original = "Texte médical exact, intégral.".to_owned();
        assert_eq!(preserve_transcript_on_structure_failure(original.clone(), Err("offline".into())), original);
        assert_eq!(preserve_transcript_on_structure_failure(original.clone(), Ok(String::new())), original);
        assert_eq!(preserve_transcript_on_structure_failure(original.clone(), Ok("Compte rendu IA".into())), "Compte rendu IA");
    }

    #[test]
    fn long_pcm_capture_is_streamed_to_a_wav_file_without_duration_cap() {
        let path = std::env::temp_dir().join(format!("scribewatch-test-{}.wav", Uuid::new_v4()));
        let mut sink = WavSink::create(&path).unwrap();
        for _ in 0..1000 {
            sink.process_chunk(AudioChunk {
                samples: &[0.25f32; 480],
                sample_rate: 16_000,
                channels: 1,
                valid_frames: 480,
            });
        }
        assert_eq!(sink.finish().unwrap(), 480_000);
        let mut decoder = hound::WavReader::open(&path).unwrap();
        assert_eq!(decoder.spec().sample_rate, 16000);
        assert_eq!(decoder.len(), 480_000);
        assert_eq!(decoder.samples::<i16>().next().unwrap().unwrap(), 8192);
        fs::remove_file(&path).unwrap();
    }
}
