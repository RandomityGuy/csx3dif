use std::collections::HashMap;
use std::thread;
use std::time::Instant;

use clap::Parser;
use clap::ValueEnum;
use csx::bsp::SplitMethod;
use csx::builder::ProgressEventListener;
use csx::convert_csx_to_dif;
use csx::set_convert_configuration;
use dif::io::EngineVersion;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BSPAlgo {
    Sampling,
    Exhaustive,
    None,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum EngineVer {
    MBG,
    TGE,
    TGEA,
    T3D,
}

impl Into<EngineVersion> for EngineVer {
    fn into(self) -> EngineVersion {
        match self {
            EngineVer::MBG => EngineVersion::MBG,
            EngineVer::TGE => EngineVersion::TGE,
            EngineVer::TGEA => EngineVersion::TGEA,
            EngineVer::T3D => EngineVersion::T3D,
        }
    }
}

impl Into<SplitMethod> for BSPAlgo {
    fn into(self) -> SplitMethod {
        match self {
            BSPAlgo::Exhaustive => SplitMethod::Exhaustive,
            BSPAlgo::Sampling => SplitMethod::Fast,
            BSPAlgo::None => SplitMethod::None,
        }
    }
}

#[derive(Parser)]
#[command(name = "csx3dif")]
#[command(author = "RandomityGuy")]
#[command(version = "1.0.8")]
#[command(about = "Convert Torque Constructor CSX files to Torque DIF files easily!")]
struct Args {
    filepath: String,
    #[arg(
        short,
        long,
        help = "Silent, don't print output",
        default_value = "false"
    )]
    silent: bool,
    #[arg(short, long, value_parser = clap::value_parser!(u32).range(0..14), help = "Dif version to export to", default_value = "0")]
    dif_version: Option<u32>,
    #[arg(
        value_enum,
        short,
        long,
        help = "Engine version to export to",
        default_value = "mbg"
    )]
    engine_version: Option<EngineVer>,
    #[arg(
        long,
        help = "Make DIF optimized for Marble Blast",
        default_value = "true"
    )]
    mb: Option<bool>,
    #[arg(
        value_enum,
        long,
        help = "BSP algorithm to use",
        default_value = "exhaustive"
    )]
    bsp: Option<BSPAlgo>,
    #[arg(
        long,
        help = "Epsilon for points to be considered the same",
        default_value = "0.000001"
    )]
    epsilon_point: Option<f32>,
    #[arg(
        long,
        help = "Epsilon for planes to be considered the same",
        default_value = "0.00001"
    )]
    epsilon_plane: Option<f32>,
}

struct ConsoleProgressListener {
    thread_tx: Option<std::sync::mpsc::Sender<(bool, u32, u32, String, String)>>,
}

impl ConsoleProgressListener {
    fn new() -> Self {
        ConsoleProgressListener { thread_tx: None }
    }
    fn init(&mut self) -> thread::JoinHandle<()> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.thread_tx = Some(sender);
        let handler: thread::JoinHandle<_> = thread::spawn(move || {
            let progress_bar: MultiProgress = MultiProgress::new();
            let mut progress_types: HashMap<String, (ProgressBar, Instant)> = HashMap::new();
            loop {
                let (stop, current, total, status, finish_status): (
                    bool,
                    u32,
                    u32,
                    String,
                    String,
                ) = receiver.recv().unwrap();
                if stop {
                    break;
                }
                if total == 0 {
                    progress_bar.println(status).unwrap();
                    progress_bar.clear().unwrap();
                } else if let Some((bar, ref mut last_updated)) = progress_types.get_mut(&status) {
                    let recvtime = std::time::Instant::now();
                    if recvtime.duration_since(*last_updated).as_millis() < 100 && total != current
                    {
                        continue;
                    }
                    *last_updated = recvtime;

                    bar.set_length(total as u64);
                    bar.set_position(current as u64);
                    bar.set_message(status.clone());
                    if current == total {
                        bar.finish_with_message(finish_status);
                        // self.progress_types.remove(&status);
                    }
                } else {
                    let sty =
                        ProgressStyle::with_template("{msg} {bar:40.cyan/blue} {pos:>7}/{len:7}")
                            .unwrap();
                    let pbar = progress_bar.add(ProgressBar::new(total as u64));
                    pbar.set_style(sty);
                    pbar.set_position(current as u64);
                    pbar.set_message(status.clone());
                    progress_types.insert(status.clone(), (pbar, std::time::Instant::now()));
                }
            }
        });
        handler
    }

    fn stop(&self) {
        self.thread_tx
            .as_ref()
            .unwrap()
            .send((true, 0, 0, "".to_owned(), "".to_owned()))
            .unwrap();
    }
}

impl ProgressEventListener for ConsoleProgressListener {
    fn progress(&mut self, current: u32, total: u32, status: String, finish_status: String) {
        self.thread_tx
            .as_ref()
            .unwrap()
            .send((false, current, total, status, finish_status))
            .unwrap();
    }
}

struct SilentListener {}

impl ProgressEventListener for SilentListener {
    fn progress(&mut self, _: u32, _: u32, _: String, _: String) {}
}

fn main() {
    let args = Args::parse();
    let filepath = &args.filepath;
    println!("Converting {}", filepath);

    let mut listener = ConsoleProgressListener::new();
    let mut silent_listener = SilentListener {};
    let join_handler = listener.init();

    let listener_to_pass: &mut dyn ProgressEventListener = if args.silent {
        &mut silent_listener
    } else {
        &mut listener
    };

    let reader = std::fs::read_to_string(filepath).unwrap();
    unsafe {
        set_convert_configuration(
            args.mb.unwrap(),
            args.epsilon_point.unwrap(),
            args.epsilon_plane.unwrap(),
            args.bsp.unwrap().into(),
        );
    }
    let ret_path = std::path::Path::new(&args.filepath)
        .with_extension("")
        .into_os_string()
        .into_string()
        .unwrap();
    let (buf, reports) = convert_csx_to_dif(
        reader,
        args.engine_version.unwrap().into(),
        args.dif_version.unwrap(),
        listener_to_pass,
    );
    buf.iter().enumerate().for_each(|(i, b)| {
        if i == 0 {
            std::fs::write(format!("{}.dif", ret_path), b).unwrap();
        } else {
            std::fs::write(format!("{}-{}.dif", ret_path, i), b).unwrap();
        }
    });
    listener.stop();
    join_handler.join().unwrap();
    // Write the reports
    reports.iter().enumerate().for_each(|(i, r)| {
        println!("BSP Report {}", i + 1);
        println!(
            "Raycast Coverage: {}/{} ({}% of surface area)",
            r.hit, r.total, r.hit_area_percentage
        );
        println!("Balance Factor: {}", r.balance_factor);
    });
}
