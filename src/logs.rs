use std::{
    io::{
        Write,
        self
    }
};
use colored::{
    *
};
use log::{
    Record,
    Level
};
use pretty_env_logger::{
    env_logger::{
        fmt::{
            Formatter
        }
    }
};
use crate::{
    app_arguments::{
        VerboseLevel
    }
};

fn log_write(buf: &mut Formatter, rec: &Record) -> io::Result<()> {
    match rec.level(){
        Level::Info => {
            writeln!(buf, "{}", rec.args())
        },
        Level::Debug => {
            let prefix = "DEBUG".green();
            writeln!(buf, "{}: {}", prefix, rec.args())
        },
        Level::Trace => {
            let prefix = "TRACE".blue();
            writeln!(buf, "{}: {}", prefix, rec.args())
        },
        Level::Warn => {
            let prefix = "DEBUG".yellow();
            writeln!(buf, "{}: {}", prefix, rec.args())
        },
        Level::Error => {
            let prefix = "ERROR".red();
            writeln!(buf, "{}: {}", prefix, rec.args())
        }
    }
}

pub fn setup_logs(verbose: &VerboseLevel){
    match verbose {
        VerboseLevel::None => {
            pretty_env_logger::formatted_builder()
                .format(log_write)
                .filter_module("m3u8_downloader", log::LevelFilter::Info)
                .init();
        },
        VerboseLevel::Medium => {
            pretty_env_logger::formatted_builder()
                .filter_module("m3u8_downloader", log::LevelFilter::Debug)
                .format(log_write)
                .init();
        },
        VerboseLevel::Max => {
            pretty_env_logger::formatted_builder()
                .filter_module("m3u8_downloader", log::LevelFilter::Trace)
                .format(log_write)
                .init();
        }
    }
}