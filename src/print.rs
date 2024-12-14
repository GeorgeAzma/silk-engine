use lazy_static::lazy_static;
use std::io::{Read, Seek, Write};

pub fn col(text: &str, col: [u8; 3]) -> String {
    format!("\x1b[38;2;{};{};{}m{text}\x1b[0m", col[0], col[1], col[2])
}

pub fn bg_col(text: &str, col: [u8; 3]) -> String {
    format!("\x1b[48;2;{};{};{}m{text}\x1b[0m", col[0], col[1], col[2])
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
    col(text, [150, 150, 150])
}

lazy_static! {
    pub static ref INIT_LOG_FOLDER: () = {
        #[cfg(debug_assertions)]
        std::fs::remove_dir_all("logs").unwrap_or_default();
        #[cfg(debug_assertions)]
        std::fs::create_dir("logs").unwrap_or_default();
    };
}

pub fn backtrace() -> String {
    *INIT_LOG_FOLDER;
    let mut backtrace = std::backtrace::Backtrace::force_capture()
        .to_string()
        .replace("\\", "/");
    backtrace = backtrace
        .lines()
        .filter(|l| l.contains("at ./src/"))
        .collect();
    backtrace = backtrace.trim().replace("at ./", "");
    let mut callers = backtrace
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .rev()
        .collect::<Vec<&str>>();
    callers.pop();
    callers.pop();
    callers.dedup();
    backtrace = callers.join(" > ");
    backtrace
}

const LOG_PATH: &str = "logs/debug.log";
const LOG_SIZE: usize = 65536;
pub fn log(text: &str) {
    #[cfg(not(debug_assertions))]
    return;

    *INIT_LOG_FOLDER;
    if let Ok(mut log_file) = std::fs::OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(LOG_PATH)
    {
        log_file
            .write_fmt(format_args!("{text}\n"))
            .unwrap_or_default();
        if log_file.metadata().unwrap().len() >= LOG_SIZE as u64 {
            let mut buf = vec![0; LOG_SIZE / 2];
            log_file
                .seek(std::io::SeekFrom::Start(LOG_SIZE as u64 / 2))
                .unwrap_or_default();
            log_file.read(&mut buf).unwrap_or_default();
            std::fs::write(LOG_PATH, &buf).unwrap_or_default();
        }
    }
}
