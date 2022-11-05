use directories::{self, UserDirs};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
// use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
// use std::process::Command;

#[derive(Debug)]
pub struct CrossCommand {
    program: String,
    args: Vec<String>,
}

pub trait CommandTrait<'b, T, E> {
    fn new(path: &str) -> Self;
    fn args<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>;
    fn output(self) -> Result<T, E>;
    fn get_stdout(value: T) -> String;
}

#[derive(Serialize, Deserialize)]
/// file path is the full path inluding the video name, and output_dir is only the output dir
pub struct OutFile {
    pub full_path: String,
    pub explorer_dir: String,
}

pub struct Converter<'a, C, T, E> {
    pub ffmpeg: Box<&'a Path>,
    pub ffprobe: Box<&'a Path>,
    pub input: Box<&'a Path>,
    pub command_phantom_data: PhantomData<C>,
    pub process_phantom_data: PhantomData<T>,
    pub error_phantom_data: PhantomData<E>,
}

impl<'a, C, T, E> Converter<'a, C, T, E>
where
    C: CommandTrait<'a, T, E>,
    E: Error,
{
    pub fn new(ffmpeg: &'a Path, ffprobe: &'a Path, input: &'a Path) -> Self {
        Converter {
            ffmpeg: Box::new(ffmpeg),
            ffprobe: Box::new(ffprobe),
            input: Box::new(input),
            command_phantom_data: PhantomData,
            process_phantom_data: PhantomData,
            error_phantom_data: PhantomData,
        }
    }

    pub fn get_output(&self) -> PathBuf {
        let user_dirs = UserDirs::new().expect("Failed to find user dirs");

        let vid_dir = match user_dirs.video_dir() {
            Some(vid_dir) => vid_dir.as_os_str().to_str().unwrap(),
            _ => {
                // if video dir fails, use the parent dir of the clip
                match self.input.parent() {
                    Some(dir) => dir.as_os_str().to_str().unwrap(),
                    // use current dir
                    _ => ".",
                }
            }
        };

        let file_name = match &self.input.file_stem() {
            Some(name) => name.to_str().unwrap(),
            _ => {
                panic!("No file name")
            }
        };

        let file_out = format!("{}-8m.mp4", file_name);
        let output_path = Path::new(vid_dir).join(file_out);

        output_path
    }

    pub fn convert_out(&self, video_bitrate: f32, audio_bitrate: f32, f_output: &Path) {
        let temp_dir = env::temp_dir();

        let output = C::new(self.ffmpeg.to_str().unwrap())
            // .expect("failed to get ffmpeg sidecar")
            .args([
                "-y",
                "-i",
                self.input.to_str().unwrap(),
                "-c:v",
                "libx264",
                "-passlogfile",
                temp_dir
                    .to_str()
                    .expect("Failed to convert temp dir to string"),
                "-filter:v",
                "scale=1280:-1",
                "-b:v",
                format!("{}k", video_bitrate).as_str(),
                "-pass",
                "2",
                "-c:a",
                "aac",
                "-b:a",
                format!("{}k", audio_bitrate).as_str(),
                f_output.to_str().unwrap(),
            ])
            .output()
            .expect("Failed first conversion");
        // let _err = String::from_utf8_lossy(&output.stderr);
        // let _out = String::from_utf8_lossy(&output.stdout);
    }

    pub fn convert_first(&self, video_bitrate: f32) {
        let temp_dir = env::temp_dir();
        let nul = if env::consts::OS == "windows" {
            "nul"
        } else {
            "/dev/null"
        };

        // make 1280:-1 conditional if video is already smaller than that
        let output = C::new(self.ffmpeg.to_str().unwrap())
            // .expect("failed to get ffmpeg sidecar")
            .args([
                "-y",
                "-i",
                self.input.to_str().unwrap(),
                "-c:v",
                "libx264",
                "-passlogfile",
                temp_dir
                    .to_str()
                    .expect("Failed to convert temp dir to string"),
                "-filter:v",
                "scale=1280:-1",
                "-b:v",
                format!("{}k", video_bitrate).as_str(),
                "-pass",
                "1",
                "-an",
                "-f",
                "mp4",
                nul,
            ])
            .output()
            .expect("Failed first conversion");

        // let err = String::from_utf8_lossy(&output.stderr);
        // let out = String::from_utf8_lossy(&output.stdout);

        // println!("{}, {}", err, out);
    }

    /// returns in kib/s
    pub fn get_target_video_rate(&self, size: f32, duration: f32, audio_rate: f32) -> f32 {
        let size = (size * 8192.00) / (1.048576 * duration) - audio_rate;
        size
    }

    pub fn get_target_size(&self, audio_rate: f32, duration: f32) -> f32 {
        let size = (audio_rate * duration) / 8192.00;
        size
    }

    /// Returns in kb
    pub fn get_original_audio_rate(&self) -> f32 {
        let ffprobe = self.ffprobe.to_str().unwrap();
        let out = C::new(ffprobe)
            // .expect("failed to find ffprobe sidecar")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=bit_rate",
                "-of",
                "csv=p=0",
                self.input.to_str().unwrap(),
            ])
            .output()
            .expect("Failed to run ffprobe to get original audio rate");

        let output = C::get_stdout(out);

        let arate = remove_whitespace(&output);

        if arate == "N/A" {
            return 0.00;
        }

        let parsed: f32 = arate
            .parse::<f32>()
            .expect("Failed to parse original audio rate")
            / 1024.00;

        parsed
        // use 7.8
    }

    // copy ffmpeg-adsf to ffmpeg
    pub fn get_duration(&self) -> f32 {
        let ffprobe = self.ffprobe.to_str().unwrap();
        let output = C::new(ffprobe)
            // .expect("failed to find ffprobe sidecar")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "csv=p=0",
                self.input.to_str().unwrap(),
            ])
            // TODO: write custom error handler
            .output()
            .expect("Failed to run ffprobe to get duration");

        // let output = Command::new(ffprobe)
        //     .output()
        //     .expect("failed to execute process");

        // println!("teste: {}", String::from_utf8_lossy(&output.stdout));
        // println!("error: {}", String::from_utf8_lossy(&output.stderr));
        let output = C::get_stdout(output);

        let duration = remove_whitespace(&output);

        let parsed: f32 = duration.parse().unwrap();

        parsed
    }
}

impl OutFile {
    pub fn new(file_path: String, output_dir: String) -> Self {
        OutFile {
            full_path: file_path,
            explorer_dir: output_dir,
        }
    }

    pub fn empty() -> Self {
        OutFile {
            full_path: "".to_string(),
            explorer_dir: "".to_string(),
        }
    }
}

fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

pub fn get_target_size(audio_rate: f32, duration: f32) -> f32 {
    let size = (audio_rate * duration) / 8192.00;
    size
}

pub fn is_minsize(min_size: f32, size: f32) -> bool {
    return min_size < size;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
