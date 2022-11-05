use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use atem_converter::{is_minsize, CommandTrait, Converter};
use clap::{arg, command, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: String,
    #[arg(short, long)]
    output: Option<String>,
    #[arg(long, default_value = "ffmpeg.exe")]
    ffmpeg: Option<String>,
    #[arg(long, default_value = "ffprobe.exe")]
    ffprobe: Option<String>,
    #[arg(short, long, default_value = "15.5")]
    target_size: Option<f32>,
}

fn get_current_working_dir() -> std::io::Result<PathBuf> {
    env::current_dir()
}

#[derive(Debug)]
pub struct ProcessCommandCrossCommand {
    program: String,
    args: Vec<String>,
}

impl<'b> CommandTrait<'b, std::process::Output, std::io::Error> for ProcessCommandCrossCommand {
    fn new(path: &str) -> Self {
        Self {
            program: path.to_string(),
            args: Vec::new(),
        }
    }

    #[must_use]
    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_string());
        }
        self
    }

    fn output(self) -> std::io::Result<std::process::Output> {
        Command::new(self.program).args(self.args).output()
    }

    fn get_stdout(value: std::process::Output) -> String {
        String::from_utf8_lossy(&value.stdout).to_string()
    }
}

fn main() {
    let args = Args::parse();

    let input = args.input;
    let ffmpegstr = match args.ffmpeg {
        Some(v) => v,
        None => "ffmpeg.exe".to_string(),
    };

    let ffprobestr = match args.ffprobe {
        Some(v) => v,
        None => "ffprobe.exe".to_string(),
    };

    let ffmpeg = Path::new(&ffmpegstr);

    let ffprobe = Path::new(&ffprobestr);

    let target_size: f32 = match args.target_size {
        Some(v) => v,
        None => 15.5,
    };

    let path_input = Path::new(&input);

    let converter: Converter<ProcessCommandCrossCommand, std::process::Output, std::io::Error> =
        Converter::new(ffmpeg, ffprobe, path_input);

    let outstr = match args.output {
        Some(v) => v,
        None => {
            let inp = Path::new(&input);
            let ext = match inp.extension() {
                Some(v) => v.to_str().unwrap(),
                None => "",
            };
            let filename = match inp.file_stem() {
                Some(v) => v.to_str().unwrap(),
                None => "",
            };

            let nome = String::new() + filename + "_out." + ext;
            nome
        }
    };
    let cwd = get_current_working_dir().unwrap();
    let cwd_with_file = Path::new(&cwd).join(&outstr);
    let cwd_with_file_as_path = &cwd_with_file.as_path();

    let caminho = if Path::new(&outstr).has_root() {
        Path::new(&outstr)
    } else {
        cwd_with_file_as_path
    };

    let duration = converter.get_duration();
    let audio_rate = converter.get_original_audio_rate();
    let min_size = converter.get_target_size(audio_rate, duration);

    if !is_minsize(min_size, target_size) {
        println!("{min_size}");
        return;
    }

    let target_bitrate = converter.get_target_video_rate(target_size, duration, audio_rate);
    converter.convert_first(target_bitrate);

    let output_path = Path::new(&caminho);
    converter.convert_out(target_bitrate, audio_rate, output_path);
}
