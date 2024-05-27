use std::{io::{BufRead, BufReader, Write}, net::{TcpListener, TcpStream}, sync::mpsc::Sender};

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
    /// Number of channels for each input/output. Defaults to 2 (stereo)
    #[arg(long, short, default_value_t = 2)]
    channels: usize,

    /// Repeat this argument multiple times to specify the names of each input
    #[arg(long = "input", short)]
    inputs: Vec<String>,

    /// Name of the client when connecting to JACK
    #[arg(long, short = 'n')]
    client_name: Option<String>,

    /// Port for the control server to listen on
    #[arg(long, short = 'p')]
    listen_port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let (client, _status) = jack::Client::new(&args.client_name.unwrap_or_else(|| "selector".to_owned()), jack::ClientOptions::NO_START_SERVER).unwrap();
    let sample_rate = client.sample_rate();
    let sample_t = 1.0 / sample_rate as f64;
    let mut sources = Vec::<Vec<Port<AudioIn>>>::with_capacity(args.inputs.len());
    let mut outputs = Vec::<Port<AudioOut>>::with_capacity(args.channels);

    for channel in 0..args.channels {
        outputs.push(client.register_port(&format!("out_{channel}"), jack::AudioOut::default()).expect("failed to create output channel"));
        let mut source_ports = Vec::with_capacity(args.channels);
        for source in &args.inputs {
            source_ports.push(client.register_port(&format!("{source}_{channel}"), jack::AudioIn::default()).expect("failed to create input channel"));
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
                for (j, (&s1, &s2)) in sources[i][previous_src].as_slice(ps).iter().zip(sources[i][new_src].as_slice(ps)).enumerate() {
                    let interpolate_progress = interpolate_time / target_duration;
                    if interpolate_progress >= target_duration {
                        out_slice[j] = s2;
                        previous_src = new_src;
                    } else {
                        out_slice[j] = do_interpolate(interpolate_progress, s1, s2);
                    }
                    interpolate_time += sample_t;
                }
            }
            jack::Control::Continue
        },
    );

    let _active_client = client.activate_async((), process).unwrap();

    let listener = TcpListener::bind(format!("::1:{}", args.listen_port))?;
    for conn in listener.incoming() {
        match conn {
            Ok(conn) => {
                let tx = tx.clone();
                let inputs = args.inputs.clone();
                std::thread::spawn(move || match handle_connection(&inputs, conn, tx) {
                    Ok(()) => {},
                    Err(e) => eprintln!("error in connection: {e}"),
                });
            },
            Err(e) => eprintln!("error on connection: {e}"),
        }
    }

    Ok(())
}

fn handle_connection(inputs: &[String], mut conn: TcpStream, tx: Sender<usize>) -> Result<(), std::io::Error> {
    let reader = BufReader::new(conn.try_clone()?);
    let mut lines = reader.lines();
    while let Some(Ok(line)) = lines.next() {
        if let Some(index) = inputs.iter().position(|s| s == &line) {
            tx.send(index).expect("failed to switch source");
            println!("source changed to {line} by {}", conn.peer_addr().unwrap());
            write!(conn, "{line}\n")?;
        } else {
            write!(conn, "invalid source\n")?;
        }
    }
    Ok(())
}
