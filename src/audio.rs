use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::fs::File;
use std::sync::{Arc, Mutex};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioBackend {
    device: Device,
    config: StreamConfig,
    stream: Option<Stream>,
    state: Arc<Mutex<AudioState>>,
    prebuffer_packets: usize,
}

struct AudioState {
    playing: bool,
    volume: f32,
    decoder: Option<Box<dyn Decoder>>,
    format: Option<Box<dyn FormatReader>>,
    sample_buffer: Vec<f32>,
    buffer_position: usize,
}

impl AudioBackend {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_prebuffer(50)
    }

    pub fn with_prebuffer(prebuffer_packets: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config = device.default_output_config()?.into();

        let state = Arc::new(Mutex::new(AudioState {
            playing: false,
            volume: 0.5,
            decoder: None,
            format: None,
            sample_buffer: Vec::new(),
            buffer_position: 0,
        }));

        Ok(Self {
            device,
            config,
            stream: None,
            state,
            prebuffer_packets,
        })
    }

    pub fn load_track(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("[Audio Backend] Loading track: {}", path);

        let file = Box::new(File::open(path)?);
        let mss = MediaSourceStream::new(file, Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path).extension() {
            hint.with_extension(ext.to_str().unwrap_or(""));
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let mut format = probed.format;
        let track = format.default_track().ok_or("No default track found")?;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

        let mut initial_samples = Vec::new();
        let mut packets_decoded = 0;

        while packets_decoded < self.prebuffer_packets {
            match format.next_packet() {
                Ok(packet) => match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let spec = *decoded.spec();
                        let duration = decoded.capacity() as u64;
                        let mut buf = SampleBuffer::<f32>::new(duration, spec);
                        buf.copy_interleaved_ref(decoded);
                        initial_samples.extend_from_slice(buf.samples());
                        packets_decoded += 1;
                    }
                    Err(_) => continue,
                },
                Err(_) => break,
            }
        }

        let mut state = self.state.lock().unwrap();
        state.decoder = Some(decoder);
        state.format = Some(format);
        state.sample_buffer = initial_samples;
        state.buffer_position = 0;

        println!(
            "[Audio Backend] Track loaded with {} pre-buffered samples",
            state.sample_buffer.len()
        );

        Ok(())
    }

    pub fn play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[Audio Backend] Starting playback");

        let state = Arc::clone(&self.state);

        {
            let mut s = state.lock().unwrap();
            s.playing = true;
        }

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut state = state.lock().unwrap();

                if !state.playing {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }

                for sample in data.iter_mut() {
                    while state.buffer_position >= state.sample_buffer.len() {
                        let has_decoder = state.decoder.is_some() && state.format.is_some();
                        if !has_decoder {
                            state.playing = false;
                            *sample = 0.0;
                            return;
                        }

                        let packet_result = state.format.as_mut().unwrap().next_packet();

                        match packet_result {
                            Ok(packet) => match state.decoder.as_mut().unwrap().decode(&packet) {
                                Ok(decoded) => {
                                    let spec = *decoded.spec();
                                    let duration = decoded.capacity() as u64;
                                    let mut buf = SampleBuffer::<f32>::new(duration, spec);
                                    buf.copy_interleaved_ref(decoded);

                                    state.sample_buffer.clear();
                                    state.sample_buffer.extend_from_slice(buf.samples());
                                    state.buffer_position = 0;
                                }
                                Err(_) => {
                                    state.playing = false;
                                    *sample = 0.0;
                                    return;
                                }
                            },
                            Err(_) => {
                                state.playing = false;
                                *sample = 0.0;
                                return;
                            }
                        }
                    }

                    if state.buffer_position < state.sample_buffer.len() {
                        *sample = state.sample_buffer[state.buffer_position] * state.volume;
                        state.buffer_position += 1;
                    } else {
                        *sample = 0.0;
                    }
                }
            },
            |err| eprintln!("[Audio Backend] Stream error: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);

        Ok(())
    }

    pub fn pause(&mut self) {
        println!("[Audio Backend] Pausing playback");
        let mut state = self.state.lock().unwrap();
        state.playing = false;
    }

    pub fn stop(&mut self) {
        println!("[Audio Backend] Stopping playback");
        let mut state = self.state.lock().unwrap();
        state.playing = false;
        state.buffer_position = 0;
        state.sample_buffer.clear();
    }

    pub fn set_volume(&mut self, volume: f32) {
        println!("[Audio Backend] Setting volume to {}", volume);
        let mut state = self.state.lock().unwrap();
        state.volume = volume.clamp(0.0, 1.0);
    }

    pub fn is_playing(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.playing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_backend_creation() {
        if let Ok(backend) = AudioBackend::new() {
            assert!(!backend.is_playing());
        }
    }

    #[test]
    fn test_volume_clamping() {
        if let Ok(mut backend) = AudioBackend::new() {
            backend.set_volume(1.5);
            let state = backend.state.lock().unwrap();
            assert_eq!(state.volume, 1.0);
        }
    }
}
