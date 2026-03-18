use cpal::traits::DeviceTrait;
use cpal::{self, FromSample, Sample};
use crate::{AUDIO_BUFFER, SAMPLE_RATE};

pub fn capture_output_audio(device: &cpal::Device) -> Option<cpal::Stream> {
    println!(
        "Capturing audio from: {}",
        device
            .name()
            .unwrap_or_else(|_| "Unknown".to_string())
    );
    let audio_cfg = device
        .default_output_config()
        .expect("No default output config found");
    println!("Default audio {:?}", audio_cfg);
    let sample_rate = audio_cfg.sample_rate().0;
    unsafe { SAMPLE_RATE = sample_rate as f32 };
    
    match audio_cfg.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(device, &audio_cfg.config()),
        cpal::SampleFormat::I16 => build_stream::<i16>(device, &audio_cfg.config()),
        cpal::SampleFormat::U16 => build_stream::<u16>(device, &audio_cfg.config()),
        _ => {
            println!("Unsupported sample format");
            None
        }
    }
}

fn build_stream<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Option<cpal::Stream>
where
    T: Sample + cpal::SizedSample + 'static,
    f32: FromSample<T>,
{
    match device.build_input_stream(
        config,
        move |data: &[T], _: &_| wave_reader(data),
        capture_err_fn,
        None,
    ) {
        Ok(stream) => Some(stream),
        Err(e) => {
            println!("Error capturing audio stream: {}", e);
            None
        }
    }
}

fn capture_err_fn(err: cpal::StreamError) {
    println!("Error {} building audio input stream", err);
}

fn wave_reader<T>(samples: &[T])
where
    T: Sample,
    f32: FromSample<T>,
{
    let mut buffer = AUDIO_BUFFER.lock();
    for i in 0..samples.len() / 2 {
        let s1: f32 = samples[2 * i].to_sample();
        let s2: f32 = samples[2 * i + 1].to_sample();
        let avg = (s1 + s2) / 2.0;
        buffer.push_back(avg);
    }
}