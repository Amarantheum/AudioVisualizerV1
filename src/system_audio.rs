use cpal::traits::DeviceTrait;
use cpal;
use crate::{AUDIO_BUFFER, SAMPLE_RATE};

pub fn capture_output_audio(device: &cpal::Device) -> Option<cpal::Stream> {
    println!(
        "Capturing audio from: {}",
        device
            .name()
            .expect("Could not get default audio device name")
    );
    let audio_cfg = device
        .default_output_config()
        .expect("No default output config found");
    println!("Default audio {:?}", audio_cfg);
    let sample_rate = audio_cfg.sample_rate().0;
    unsafe { SAMPLE_RATE = sample_rate as f32 };
    match audio_cfg.sample_format() {
        cpal::SampleFormat::F32 => match device.build_input_stream(
            &audio_cfg.config(),
            move |data, _: &_| wave_reader::<f32>(data),
            capture_err_fn,
        ) {
            Ok(stream) => Some(stream),
            Err(e) => {
                println!("Error capturing f32 audio stream: {}", e);
                None
            }
        },
        cpal::SampleFormat::I16 => {
            match device.build_input_stream(
                &audio_cfg.config(),
                move |data, _: &_| wave_reader::<i16>(data),
                capture_err_fn,
            ) {
                Ok(stream) => Some(stream),
                Err(e) => {
                    println!("Error capturing i16 audio stream: {}", e);
                    None
                }
            }
        }
        cpal::SampleFormat::U16 => {
            match device.build_input_stream(
                &audio_cfg.config(),
                move |data, _: &_| wave_reader::<u16>(data),
                capture_err_fn,
            ) {
                Ok(stream) => Some(stream),
                Err(e) => {
                    println!("Error capturing u16 audio stream: {}", e);
                    None
                }
            }
        }
    }
}

/// capture_err_fn - called whan it's impossible to build an audio input stream
fn capture_err_fn(err: cpal::StreamError) {
    println!("Error {} building audio input stream", err);
}

fn wave_reader<T>(samples: &[T])
where
    T: cpal::Sample + std::fmt::Debug,
{
    let mut buffer = AUDIO_BUFFER.lock();
    for i in 0..samples.len() / 2 {
        let avg = (samples[2 * i].to_f32() + samples[2 * i + 1].to_f32()) / 2_f32;
        buffer.push_back(avg);
    }
}