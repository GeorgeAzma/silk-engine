use std::{
    fs::File,
    io::{Read, Seek, Write},
    sync::Mutex,
};

pub fn col(text: &str, col: [u8; 3]) -> String {
    format!("\x1b[38;2;{};{};{}m{text}\x1b[0m", col[0], col[1], col[2])
}

pub fn bg_col(text: &str, col: [u8; 3]) -> String {
    format!("\x1b[48;2;{};{};{}m{text}\x1b[0m", col[0], col[1], col[2])
}

pub fn dim(text: &str) -> String {
    format!("\x1b[2m{text}\x1b[0m")
}

pub const FATAL_RGB: [u8; 3] = [241, 76, 76];
pub const ERR_RGB: [u8; 3] = [241, 76, 76];
pub const WARN_RGB: [u8; 3] = [245, 245, 67];
pub const SUCCESS_RGB: [u8; 3] = [35, 209, 139];
pub const INFO_RGB: [u8; 3] = [41, 184, 219];

pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const TRACE: &str = DIM;
pub const INFO: &str = "\x1b[38;2;41;184;219m";
pub const SUCCESS: &str = "\x1b[38;2;35;209;139m";
pub const WARN: &str = "\x1b[38;2;245;245;67m";
pub const ERR: &str = "\x1b[38;2;241;76;76m";
pub const FATAL: &str = "\x1b[38;2;241;76;76m";

pub enum Level {
    Trace,
    Debug,
    Info,
    Success,
    Warn,
    Error,
    Fatal,
}

pub struct Record<'a> {
    pub level: Level,
    pub msg: std::fmt::Arguments<'a>,
}

pub trait Sink: Send + Sync {
    fn name(&self) -> &'static str {
        ""
    }
    fn write(&mut self, record: &Record);
}

pub struct ConsoleSink;
impl Sink for ConsoleSink {
    fn name(&self) -> &'static str {
        "console"
    }

    fn write(&mut self, record: &Record) {
        let mut stdout = std::io::stdout();
        let msg = record.msg;
        match record.level {
            Level::Trace => write!(stdout, "{}{}{}", TRACE, msg, RESET),
            Level::Debug => write!(stdout, "{}", msg),
            Level::Info => write!(stdout, "{}{}{}", INFO, msg, RESET),
            Level::Success => write!(stdout, "{}{}{}", SUCCESS, msg, RESET),
            Level::Warn => writeln!(stdout, "{}{}{}{}{}", WARN, msg, DIM, backtrace(2), RESET),
            Level::Error => writeln!(stdout, "{}{}{}{}{}", ERR, msg, DIM, backtrace(2), RESET),
            Level::Fatal => panic!("{}{}{}{}{}", FATAL, msg, DIM, backtrace(2), RESET),
        }
        .unwrap_or_default()
    }
}

pub struct RotatingFileSink {
    log_file_path: String,
    log_file: File,
    max_size: u64,
}

impl RotatingFileSink {
    pub fn new(log_file_path: &str, max_size: u64) -> Self {
        if let Some(parent) = std::path::Path::new(log_file_path).parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|err| {
                crate::warn!("failed to create directory: {log_file_path}\n{err}")
            });
        }
        Self {
            log_file_path: log_file_path.to_string(),
            log_file: std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .truncate(true)
                .create(true)
                .open(log_file_path)
                .unwrap(),
            max_size,
        }
    }

    fn write_str(&mut self, str: &str) {
        self.log_file
            .write_all(str.as_bytes())
            .unwrap_or_else(|err| {
                crate::warn!(
                    "failed logging to file {}, with string: {str}\n{err}",
                    self.log_file_path
                )
            });

        let file_len = self.log_file.metadata().unwrap().len();
        if file_len >= self.max_size {
            let mut buf = vec![0; (self.max_size / 2) as usize];
            self.log_file
                .seek(std::io::SeekFrom::End(-(self.max_size as i64 / 2)))
                .unwrap();
            self.log_file.read_exact(&mut buf).unwrap();
            self.log_file.set_len(0).unwrap();
            self.log_file.seek(std::io::SeekFrom::Start(0)).unwrap();
            self.log_file.write_all(&buf).unwrap();
        }
    }
}

impl Sink for RotatingFileSink {
    fn name(&self) -> &'static str {
        "rotating_file"
    }

    fn write(&mut self, record: &Record) {
        let msg = record.msg;
        let fmt = |level: &str| {
            let msg = format!("{msg}");
            if !msg.ends_with('\n') {
                format!("  {level} {msg}\n")
            } else {
                format!("  {level} {msg}")
            }
        };
        match record.level {
            Level::Trace => self.write_str(&fmt("[TRACE]")),
            Level::Debug => self.write_str(&fmt("[DEBUG]")),
            Level::Info => self.write_str(&fmt(" [INFO]")),
            Level::Success => self.write_str(&fmt("[SUCCS]")),
            Level::Warn => self.write_str(&fmt(" [WARN]")),
            Level::Error => self.write_str(&fmt("[ERROR]")),
            Level::Fatal => self.write_str(&fmt("[FATAL]")),
        }
    }
}

#[derive(Default)]
pub struct Logger {
    pub sinks: Vec<Box<dyn Sink>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl Logger {
    pub fn log(&mut self, record: Record) {
        for sink in &mut self.sinks {
            sink.write(&record);
        }
    }

    pub fn log_to(&mut self, sink_name: &str, record: Record) {
        for sink in &mut self.sinks {
            if sink.name() == sink_name {
                sink.write(&record);
            }
        }
    }

    pub fn log_file(&mut self, file_path: &str, str: &str) {
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|err| {
                crate::warn!("failed to create directory: {file_path}\n{err}")
            });
        }
        if let Ok(mut log_file) = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_path)
        {
            write!(log_file, "{str}").unwrap_or_default();
        }
    }
}

use std::sync::OnceLock;

pub static GLOBAL_LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

pub fn set_global_logger(logger: Logger) -> Result<(), &'static str> {
    GLOBAL_LOGGER
        .set(Mutex::new(logger))
        .map_err(|_| "Global logger is already set")
}

#[macro_export]
macro_rules! log_level {
    ($level:expr, $($args:tt)*) => {{
        $crate::util::print::GLOBAL_LOGGER
            .get()
            .expect("Global logger not set")
            .lock()
            .unwrap()
            .log($crate::util::print::Record {
                level: $level,
                msg: format_args!("{}\n", format_args!($($args)*)),
            });
    }}
}

#[macro_export]
macro_rules! log_sink {
    ($sink_name:expr, $level:expr, $($args:tt)*) => {
        $crate::util::print::GLOBAL_LOGGER
            .get()
            .expect("Global logger not set")
            .lock()
            .unwrap()
            .log_to($sink_name, $crate::util::print::Record {
                level: $level,
                msg: format_args!("{}\n", format_args!($($args)*)),
            });
    };
}

#[macro_export]
macro_rules! trace {
    ($($args:tt)*) => { $crate::log_level!($crate::util::print::Level::Trace, $($args)*) };
}

#[macro_export]
macro_rules! debug {
    ($($args:tt)*) => { $crate::log_level!($crate::util::print::Level::Debug, $($args)*) };
}

#[macro_export]
macro_rules! info {
    ($($args:tt)*) => { $crate::log_level!($crate::util::print::Level::Info, $($args)*) };
}

#[macro_export]
macro_rules! success {
    ($($args:tt)*) => { $crate::log_level!($crate::util::print::Level::Success, $($args)*) };
}

#[macro_export]
macro_rules! warn {
    ($($args:tt)*) => { $crate::log_level!($crate::util::print::Level::Warn, $($args)*) };
}

#[macro_export]
macro_rules! assert_warn {
    ($cond:expr $(,)?) => {
        if !$cond {
            $crate::warn!("Assertion failed: {}", stringify!($cond));
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        if !$cond {
            $crate::warn!($($arg)+);
        }
    };
}

#[macro_export]
macro_rules! err {
    ($($args:tt)*) => { $crate::log_level!($crate::util::print::Level::Error, $($args)*) };
}

#[macro_export]
macro_rules! fatal {
    ($($args:tt)*) => {{ $crate::log_level!($crate::util::print::Level::Fatal, $($args)*); std::process::exit(1) }};
}

#[macro_export]
macro_rules! scope_time {
    ($($args:expr),* ; $($cond:tt)+) => {
        let _t = if $($cond)+ {
            Some($crate::util::print::ScopeTime::new(&format!($($args),*)))
        } else {
            None
        };
    };
    () => {
        let _t = $crate::util::print::ScopeTime::new("");
    };
    ($($args:tt)*) => {
        let _t = $crate::util::print::ScopeTime::new(&format!($($args)*));
    };
}

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
    callers[callers.len().saturating_sub(skips + 2)].clone()
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
        if self.name.is_empty() {
            crate::trace!("[{}] {elapsed:?}", backtrace_last(1));
        } else {
            crate::trace!("[{}] {}: {elapsed:?}", backtrace_last(1), self.name);
        }
    }
}

pub fn print_rgb_pixel(rgb: [u8; 3]) {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    print!("\x1b[48;2;{r};{g};{b}m  \x1b[0m");
}

pub fn print_rgba_pixel(mut rgba: [u8; 4]) {
    let a = rgba[3] as f32 / 255.0;
    rgba[0] = (rgba[0] as f32 * a).round() as u8;
    rgba[1] = (rgba[1] as f32 * a).round() as u8;
    rgba[2] = (rgba[2] as f32 * a).round() as u8;
    print_rgb_pixel([rgba[0], rgba[1], rgba[2]]);
}

pub fn print_img(img: &[u8], width: u32, height: u32, channels: u8) {
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize * channels as usize;
            let (r, g, b) = (img[i], img[i + 1], img[i + 2]);
            if channels == 4 {
                print_rgba_pixel([r, g, b, img[i + 3]]);
            } else {
                print_rgb_pixel([r, g, b]);
            }
        }
        println!();
    }
}
