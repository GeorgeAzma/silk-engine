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
        panic!("\x1b[48;2;241;76;76m{}\x1b[0m\n\x1b[2m{}\x1b[0m", format_args!($($args)*), $crate::backtrace_skip(1))
    };
}

#[macro_export]
macro_rules! err {
    ($($args:tt)*) => {
        eprintln!("\x1b[38;2;241;76;76m{}\x1b[0m\n\x1b[2m{}\x1b[0m", format_args!($($args)*), $crate::backtrace_skip(1))
    };
}

#[macro_export]
macro_rules! err_abort {
    ($($args:tt)*) => {
        panic!("\x1b[38;2;241;76;76m{}\x1b[0m\n\x1b[2m{}\x1b[0m", format_args!($($args)*), $crate::backtrace_skip(1))
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

pub static INIT_LOG_FOLDER: LazyLock<()> = LazyLock::new(|| {
    std::fs::remove_dir_all("logs").unwrap_or_default();
    std::fs::create_dir("logs").unwrap_or_default();
});

pub fn backtrace_callers() -> Vec<String> {
    *INIT_LOG_FOLDER;
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
    callers
}

pub fn backtrace_skip(last_callers: usize) -> String {
    let mut callers = backtrace_callers();
    callers.resize(
        callers.len().saturating_sub(last_callers + 1),
        String::new(),
    );
    callers.join(" > ")
}

pub fn backtrace() -> String {
    backtrace_callers().join(" > ")
}

#[macro_export]
macro_rules! log_file {
    ($file:expr, $($args:tt)*) => {
        #[cfg(debug_assertions)]
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
        $crate::log_file!("logs/debug.log", $($args)*);
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

#[cfg(not(debug_assertions))]
pub struct ScopeTime;

#[cfg(not(debug_assertions))]
impl ScopeTime {
    pub fn new(_name: &str) -> Self {
        Self
    }
}

#[cfg(debug_assertions)]
pub struct ScopeTime {
    start: std::time::Instant,
    name: String,
}

#[cfg(debug_assertions)]
impl ScopeTime {
    pub fn new(name: &str) -> Self {
        Self {
            start: std::time::Instant::now(),
            name: name.to_string(),
        }
    }
}

#[cfg(debug_assertions)]
impl Drop for ScopeTime {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let callers = backtrace_callers();
        let caller = &callers[callers.len() - 2];
        crate::log!("[{}] {}: {:?}", caller, self.name, elapsed);
    }
}
