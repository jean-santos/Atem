use atem_converter::{is_minsize, CommandTrait, Converter};
use std::{env, path::Path};
use tauri::{
    api::{
        dialog::message,
        process::{Command, Output},
    },
    Manager,
};

#[derive(Debug)]
pub struct TauriCommandCrossCommand {
    program: String,
    args: Vec<String>,
}

impl<'b> CommandTrait<'b, Output, tauri::api::Error> for TauriCommandCrossCommand {
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

    fn output(self) -> tauri::api::Result<Output> {
        Command::new_sidecar(self.program)
            .expect("failed to find sidecar")
            .args(self.args)
            .output()
    }

    fn get_stdout(value: Output) -> String {
        value.stdout.to_string()
    }
}

#[cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#[tauri::command(async)]
fn open_file_explorer(path: &str, window: tauri::Window) {
    let label = window.label();
    let parent_window = window.get_window(label).unwrap();
    println!("{}", path);
    match env::consts::OS {
        "windows" => {
            Command::new("explorer")
                .args(["/select,", path]) // The comma after select is not a typo
                .spawn()
                .unwrap();
        }
        "macos" => {
            Command::new("open")
                .args(["-R", path]) // i don't have a mac so not 100% sure
                .spawn()
                .unwrap();
        }
        _ => {
            tauri::async_runtime::spawn(async move {
                message(
                    Some(&parent_window),
                    "Unsupported OS",
                    "Opening a file browser is unsupported on linux",
                );
            });
        }
    }
}

#[tauri::command(async)]
fn convert_video(input: &str, target_size: f32) -> String {
    let ffprobe = Path::new("ffprobe.exe");
    let ffmpeg = Path::new("ffmpeg.exe");
    let input = Path::new(input);

    let converter: Converter<TauriCommandCrossCommand, Output, tauri::api::Error> =
        Converter::new(ffmpeg, ffprobe, input);

    let output = converter.get_output();

    let duration = converter.get_duration();
    let audio_rate = converter.get_original_audio_rate();
    let min_size = converter.get_target_size(audio_rate, duration);

    if !is_minsize(min_size, target_size) {
        println!("{min_size}");
        return "".to_string();
    }

    let target_bitrate = converter.get_target_video_rate(target_size, duration, audio_rate);
    converter.convert_first(target_bitrate);
    converter.convert_out(target_bitrate, audio_rate, &output);

    let outpath = output.to_str().unwrap().to_string();

    return outpath;
}
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_file_explorer, convert_video])
        .setup(|app| {
            match app.get_cli_matches() {
                Ok(_matches) => {
                    println!("got matches");
                }
                Err(_) => {
                    println!("no matches");
                }
            };

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
