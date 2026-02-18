use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig, SupportedStreamConfig};
use ringbuf::{HeapRb, traits::*};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::*;

use crate::eq::Eq;

pub struct AudioBackend {
    device: Device,
    config: StreamConfig,
    stream: Option<Stream>,
    state: Arc<Mutex<AudioState>>,
    decoder_thread: Option<JoinHandle<()>>,
    ring_buffer_size: usize,
    eq: Arc<Mutex<Eq>>,
    producer_sleep_time: u64,
}

struct AudioState {
    playing: bool,
    volume: f32,
    stop_signal: bool,
}

// im only using ring buffer because thats the only resonable thing i could think of
// not sure if I know what im doing but it works
// also gives me more room to play with the audio without over/underruns
impl AudioBackend {
    pub fn with_ring_buffer_size(
        ring_buffer_size: usize,
        default_volume: f32,
        enable_eq: bool,
        eq_bands: Vec<[f32; 4]>,
	producer_sleep_time: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config: SupportedStreamConfig = device.default_output_config()?.into();

        let state = Arc::new(Mutex::new(AudioState {
            playing: false,
            volume: default_volume,
            stop_signal: false,
        }));

        let eq = { Eq::from_config(eq_bands.clone(), enable_eq, config.sample_rate() as f32) };

        let eq = Arc::new(Mutex::new(eq));
        Ok(Self {
            device,
            config: config.into(),
            stream: None,
            state,
            decoder_thread: None,
            ring_buffer_size,
            eq,
	    producer_sleep_time
        })
    }

    pub fn load_track(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("[Audio Backend] Loading track: {}", path);

        // kinda need to do this
        self.stop_decoder();
        let file = Box::new(File::open(path)?);

        // we let symphonia deal with the file
        let mss = MediaSourceStream::new(file, Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path).extension() {
            hint.with_extension(ext.to_str().unwrap_or(""));
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            // need to do alot with this
            &MetadataOptions::default(),
        )?;

        let format = probed.format;
        let track = format.default_track().ok_or("No default track found")?;

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

        // bridge between decoder thread and cpal callback
        // producer will write decoded samples
        // consumer will read and play
        let ring = HeapRb::<f32>::new(self.ring_buffer_size);
        let (mut producer, consumer) = ring.split();

        let state = Arc::clone(&self.state);
	let pct = self.producer_sleep_time;
        let decoder_thread = thread::spawn(move || {
            let mut decoder = decoder;
            let mut format = format;

            loop {
                {
                    let state = state.lock().unwrap();
                    if state.stop_signal {
                        break;
                    }
                }

                let packet = match format.next_packet() {
                    Ok(p) => p,
                    Err(_) => break,
                };

                let decoded = match decoder.decode(&packet) {
                    Ok(d) => d,
                    Err(_) => continue,
                };

                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                let mut buf = SampleBuffer::<f32>::new(duration, spec);
                buf.copy_interleaved_ref(decoded);

                for sample in buf.samples() {
                    while producer.try_push(*sample).is_err() {
                        // you can rest twin
                        thread::sleep(std::time::Duration::from_micros(pct));

                        let state = state.lock().unwrap();
                        if state.stop_signal {
                            return;
                        }
                    }
                }
            }

            println!("[Audio Backend] Decoder thread finished");
        });

        self.decoder_thread = Some(decoder_thread);

        let state_for_callback = Arc::clone(&self.state);
        let consumer = Arc::new(Mutex::new(consumer));
        let eq = Arc::clone(&self.eq);

        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let state = state_for_callback.lock().unwrap();
                let mut consumer = consumer.lock().unwrap();
                let mut eq = eq.lock().unwrap();
                if !state.playing {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }

                for sample in data.iter_mut() {
                    // consume and apply volume on the sample
                    // and apply eq
                    let mut s = consumer.try_pop().unwrap_or(0.0);
                    if eq.enabled {
                        s = eq.process(s);
                    }
                    *sample = s * state.volume;
                }
            },
            |err| eprintln!("[Audio Backend] Stream error: {}", err),
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);

        info!("[Audio Backend] Track loaded, decoder thread started");

        Ok(())
    }

    fn stop_decoder(&mut self) {
        if let Some(thread) = self.decoder_thread.take() {
            {
                let mut state = self.state.lock().unwrap();
                state.stop_signal = true;
            }
            thread.join().ok();
            {
                let mut state = self.state.lock().unwrap();
                state.stop_signal = false;
            }
        }
    }

    pub fn play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("[Audio Backend] Starting playback");
        let mut state = self.state.lock().unwrap();
        state.playing = true;
        Ok(())
    }

    pub fn pause(&mut self) {
        info!("[Audio Backend] Pausing playback");
        let mut state = self.state.lock().unwrap();
        state.playing = false;
    }

    pub fn stop(&mut self) {
        info!("[Audio Backend] Stopping playback");
        self.stop_decoder();
        let mut state = self.state.lock().unwrap();
        state.playing = false;
    }

    pub fn set_volume(&mut self, volume: f32) {
        info!("[Audio Backend] Setting volume to {}", volume);
        let mut state = self.state.lock().unwrap();
        state.volume = volume.clamp(0.0, 1.0);
    }

    pub fn is_playing(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.playing
    }
}

impl Drop for AudioBackend {
    fn drop(&mut self) {
        self.stop_decoder();
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
