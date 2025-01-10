use crate::RES_PATH;
use std::sync::LazyLock;

pub fn col(text: &str, col: [u8; 3]) -> String {
    format!("\x1b[38;2;{};{};{}m{text}\x1b[0m", col[0], col[1], col[2])
}

pub fn bg_col(text: &str, col: [u8; 3]) -> String {
    format!("\x1b[48;2;{};{};{}m{text}\x1b[0m", col[0], col[1], col[2])
}

pub fn dim(text: &str) -> String {
    format!("\x1b[2m{text}\x1b[0m")
}

pub fn fatal(text: &str) -> String {
    bg_col(text, [241, 76, 76])
}

pub fn err(text: &str) -> String {
    col(text, [241, 76, 76])
}

pub fn warn(text: &str) -> String {
    col(text, [245, 245, 67])
}

pub fn info(text: &str) -> String {
    col(text, [41, 184, 219])
}

pub fn trace(text: &str) -> String {
    dim(text)
}

#[macro_export]
macro_rules! fatal {
    ($($args:tt)*) => {
        panic!("\x1b[48;2;241;76;76m{}\x1b[0m\n\x1b[2m{}\x1b[0m", format_args!($($args)*), $crate::backtrace(1))
    };
}

#[macro_export]
macro_rules! err {
    ($($args:tt)*) => {
        eprintln!("\x1b[38;2;241;76;76m{}\x1b[0m\n\x1b[2m{}\x1b[0m", format_args!($($args)*), $crate::backtrace(1))
    };
}

#[macro_export]
macro_rules! warn {
    ($($args:tt)*) => {
        println!("\x1b[38;2;240;230;80m{}\x1b[0m", format_args!($($args)*))
    };
}

#[macro_export]
macro_rules! info {
    ($($args:tt)*) => {
        println!("\x1b[38;2;41;184;219m{}\x1b[0m", format_args!($($args)*))
    };
}

#[macro_export]
macro_rules! trace {
    ($($args:tt)*) => {
        println!("{}", print::trace(&format!($($args),*)))
    };
}

pub fn log_path() -> String {
    format!("{RES_PATH}/../logs")
}

pub static INIT_LOG_FOLDER: LazyLock<()> = LazyLock::new(|| {
    std::fs::remove_dir_all(log_path()).unwrap_or_default();
    std::fs::create_dir(log_path()).unwrap_or_default();
});

pub fn backtrace_callers() -> Vec<String> {
    let mut backtrace = std::backtrace::Backtrace::force_capture()
        .to_string()
        .replace("\\", "/");
    backtrace = backtrace
        .lines()
        .filter(|l| l.contains("at ./") && l.contains("/src/"))
        .collect();
    let mut callers: Vec<String> = backtrace
        .trim()
        .replace("at ./", "")
        .split_whitespace()
        .filter_map(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .rev()
        .collect();
    callers.pop();
    callers.dedup();
    if callers.is_empty() {
        callers = vec![String::new()];
    }
    callers
}

pub fn backtrace(skips: usize) -> String {
    let mut callers = backtrace_callers();
    callers.resize(callers.len().saturating_sub(skips + 1), String::new());
    callers.join(" > ")
}

pub fn backtrace_last(skips: usize) -> String {
    let callers = backtrace_callers();
    callers[callers.len().saturating_sub(2 + skips)].clone()
}

#[macro_export]
macro_rules! log_file {
    ($file:expr, $($args:tt)*) => {
        #[cfg(any(debug_assertions, test))]
        {
            use $crate::print::INIT_LOG_FOLDER;
            use std::io::{Read, Seek, Write};
            const LOG_SIZE: usize = 65536;
            *INIT_LOG_FOLDER;
            if let Ok(mut log_file) = std::fs::OpenOptions::new()
                .read(true)
                .append(true)
                .create(true)
                .open($file)
            {
                log_file
                    .write_fmt(format_args!("{}\n", format_args!($($args)*)))
                    .unwrap_or_default();
                if log_file.metadata().unwrap().len() >= LOG_SIZE as u64 {
                    let mut buf = vec![0; LOG_SIZE / 2];
                    log_file
                        .seek(std::io::SeekFrom::Start(LOG_SIZE as u64 / 2))
                        .unwrap_or_default();
                    log_file.read(&mut buf).unwrap_or_default();
                    std::fs::write($file, &buf).unwrap_or_default();
                }
            }
        }
    };
}

#[macro_export]
macro_rules! log {
    ($($args:tt)*) => {
        $crate::log_file!([crate::log_path() + "/debug.log"].concat(), $($args)*);
    }
}

#[macro_export]
macro_rules! scope_time {
    ($($args:expr),* ; $($cond:tt)+) => {
        let _t = if $($cond)+ {
            Some($crate::ScopeTime::new(&format!($($args),*)))
        } else {
            None
        };
    };
    ($($args:tt)*) => {
        let _t = $crate::ScopeTime::new(&format!($($args)*));
    };
}

#[cfg(not(any(debug_assertions, test)))]
pub struct ScopeTime;

#[cfg(not(any(debug_assertions, test)))]
impl ScopeTime {
    pub fn new(_name: &str) -> Self {
        Self
    }
}

#[cfg(any(debug_assertions, test))]
pub struct ScopeTime {
    start: std::time::Instant,
    name: String,
}

#[cfg(any(debug_assertions, test))]
impl ScopeTime {
    pub fn new(name: &str) -> Self {
        Self {
            start: std::time::Instant::now(),
            name: name.to_string(),
        }
    }
}

#[cfg(any(debug_assertions, test))]
impl Drop for ScopeTime {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        crate::log!("[{}] {}: {:?}", backtrace_last(1), self.name, elapsed);
    }
}

pub fn print_rgb(rgb: [u8; 3]) {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    print!("\x1b[48;2;{r};{g};{b}m  \x1b[0m");
}

pub fn print_rgba(mut rgba: [u8; 4]) {
    let a = rgba[3] as f32 / 255.0;
    rgba[0] = (rgba[0] as f32 * a).round() as u8;
    rgba[1] = (rgba[1] as f32 * a).round() as u8;
    rgba[2] = (rgba[2] as f32 * a).round() as u8;
    print_rgb([rgba[0], rgba[1], rgba[2]]);
}

pub fn print_img(img: &[u8], width: u32, height: u32, channels: u8) {
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize * channels as usize;
            let (r, g, b) = (img[i], img[i + 1], img[i + 2]);
            if channels == 4 {
                print_rgba([r, g, b, img[i + 3]]);
            } else {
                print_rgb([r, g, b]);
            }
        }
        println!();
    }
}
