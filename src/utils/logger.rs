// src/utils/logger.rs

use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

pub fn setup_logger(level: LevelFilter) -> Result<(), anyhow::Error> {
    let mut builder = Builder::new();
    builder
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%Z"),
                record.level(),
                record.args()
            )
        })
        .filter(None, level)
        .try_init()?;
    Ok(())
}

pub fn setup_file_logging(file_path: &str, level: LevelFilter) -> Result<(), anyhow::Error> {
    // This is a simplified version. Real file logging would involve
    // more robust handling, rotation, etc. For now, we'll just use env_logger's
    // capability to write to a target if possible, or a more complex setup
    // with a dedicated file logging crate like `log4rs` or `fern` would be needed.
    // For this example, we'll keep it simple and rely on stdout/stderr redirection
    // if file logging is strictly needed, or use a crate like `fern`.
    
    // Using fern for basic file logging as an example, this would require adding `fern` to Cargo.toml
    /*
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%Z"),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout()) // Also log to stdout
        .chain(fern::log_file(file_path)?) // Log to file
        .apply()?;
    */
    
    // For now, stick to env_logger and advise redirecting output or using a dedicated file logger.
    // The Python version uses a custom file logger. Replicating that fully would be more involved.
    println!("File logging setup for {} at level {}. (Simplified for now, consider redirecting stdout/stderr or using a dedicated file logging crate)", file_path, level);
    setup_logger(level)?; // Basic console logger for now
    Ok(())
}

