use std::io::BufRead;

use clap::Parser;
use jack::{AudioIn, AudioOut, Port};

fn interp(progress: f32) -> f32 {
    ((progress - 1.0).exp() * progress).min(1.0).max(0.0)
}

fn do_interpolate(progress: f64, s1: f32, s2: f32) -> f32 {
    (s1 * interp(1.0 - progress as f32)) + (s2 * interp(progress as f32))
}

#[derive(clap::Parser)]
struct Args {
    #[arg(long, short, default_value_t = 2)]
    channels: usize,

    #[arg(long, short)]
    inputs: usize,

    #[arg(long, short = 'n')]
    client_name: Option<String>,
}

fn main() {
    let args = Args::parse();

    let (client, _status) = jack::Client::new(&args.client_name.unwrap_or_else(|| "selector".to_owned()), jack::ClientOptions::NO_START_SERVER).unwrap();
    let sample_rate = client.sample_rate();
    let sample_t = 1.0 / sample_rate as f64;
    let mut sources = Vec::<Vec<Port<AudioIn>>>::with_capacity(args.inputs);
    let mut outputs = Vec::<Port<AudioOut>>::with_capacity(args.channels);

    for channel in 0..args.channels {
        outputs.push(client.register_port(&format!("out_{channel}"), jack::AudioOut::default()).expect("failed to create output channel"));
        let mut source_ports = Vec::with_capacity(args.channels);
        for source in 0..args.inputs {
            source_ports.push(client.register_port(&format!("in_{source}_{channel}"), jack::AudioIn::default()).expect("failed to create input channel"));
        }
        sources.push(source_ports);
    }

    let target_duration = 1.0;

    let mut previous_src = 0;
    let mut new_src = 0;
    let mut interpolate_time = target_duration;

    let (tx, rx) = std::sync::mpsc::channel::<usize>();

    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            while let Ok(new_new_src) = rx.try_recv() {
                previous_src = new_src;
                new_src = new_new_src;
                interpolate_time = 0.0;
            }
            for (i, out_port) in outputs.iter_mut().enumerate() {
                let out_slice = out_port.as_mut_slice(ps);
                for (i, (&s1, &s2)) in sources[previous_src][i].as_slice(ps).iter().zip(sources[new_src][i].as_slice(ps)).enumerate() {
                    let interpolate_progress = interpolate_time / target_duration;
                    if interpolate_progress >= target_duration {
                        out_slice[i] = s2;
                        previous_src = new_src;
                    } else {
                        out_slice[i] = do_interpolate(interpolate_progress, s1, s2);
                    }
                    interpolate_time += sample_t;
                }
            }
            jack::Control::Continue
        },
    );

    let active_client = client.activate_async((), process).unwrap();

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(Ok(new_new_src)) = line.map(|s| s.parse::<usize>()) {
            if new_new_src < args.inputs {
                tx.send(new_new_src).unwrap();
            } else {
                println!("invalid source");
            }
        } else {
            println!("invalid source");
        }
    }
}
