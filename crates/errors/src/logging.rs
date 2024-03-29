use crate::{Report, Result};
use build::{get_data_dir, LOG_ENV, LOG_FILE};

use env_logger::Builder;
use log::{debug, error, info, LevelFilter};
use std::{fmt::Debug, fs::File, io::Write, ops::Deref, panic::Location};
use sync::Mutex;

pub static WORKSPACE_CRATES: &str = env!("_WORKSPACE_CRATES");

pub fn initialize_logging() -> Result<()> {
    let directory = get_data_dir();
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(LOG_FILE.clone());

    let log_file = Box::new(Mutex::new(File::create(log_path)?));
    let log_file = Box::leak(log_file);
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG").or_else(|_| std::env::var(LOG_ENV.clone())).unwrap_or_else(|_| {
            WORKSPACE_CRATES
                .split(',')
                .map(|crate_name| format!("{}=info", crate_name.replace('-', "_")))
                .collect::<Vec<String>>()
                .join(",")
        }),
    );

    Builder::from_default_env()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| {
            let timestamp = buf.timestamp_micros();

            minitrace::Event::add_to_local_parent(record.level().as_str(), || {
                [("message".into(), record.args().to_string().into())]
            });
            writeln!(
                log_file.lock(),
                "{} {} {}:{}: {}",
                timestamp,
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
    Ok(())
}

pub trait LogErrorExt<T> {
    #[must_use]
    fn log_error(self) -> Self;
    #[must_use]
    fn log_info(self) -> Self;

    #[must_use]
    fn log_debug(self) -> Self;
}

pub trait LogErrorWithExt<T> {
    #[must_use]
    fn log_error_msg(self, message: &str) -> Self;
    #[must_use]
    fn log_info_msg(self, message: &str) -> Self;
    #[must_use]
    fn log_debug_msg(self, message: &str) -> Self;
}

pub trait LogOptionWithExt<T> {
    #[must_use]
    fn log_error_msg(self, message: &str) -> Self;
    #[must_use]
    fn log_info_msg(self, message: &str) -> Self;

    #[must_use]
    fn log_debug_msg(self, message: &str) -> Self;

    fn report_msg(self, message: &'static str) -> Result<T, Report>;
}

pub trait LogDerefWithExt<T> {
    #[must_use]
    fn log_deref_error_msg(self, message: &str) -> Self;
    #[must_use]
    fn log_deref_info_msg(self, message: &str) -> Self;
}

pub trait MakeReportExt<T, E> {
    fn report(self) -> Result<T, Report>;
    fn report_msg(self, message: &str) -> Result<T, Report>;
}

impl<T, E> MakeReportExt<T, E> for Result<T, E>
where
    E: Debug,
{
    fn report(self) -> Result<T, Report> {
        match self {
            Ok(ok) => Ok(ok),
            Err(e) => Err(Report::msg(format!("{e:?}"))),
        }
    }

    fn report_msg(self, message: &str) -> Result<T, Report> {
        match self {
            Ok(ok) => Ok(ok),
            Err(e) => Err(Report::msg(format!("{message:} {e:?}"))),
        }
    }
}

impl<T, E> LogErrorExt<T> for Result<T, E>
where
    E: Debug,
{
    #[track_caller]
    fn log_error(self) -> Self {
        if let Err(ref e) = self {
            let location = Location::caller();
            error!("{e:?} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn log_info(self) -> Self {
        if let Err(ref e) = self {
            let location = Location::caller();
            info!("{e:?} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn log_debug(self) -> Self {
        if let Err(ref e) = self {
            let location = Location::caller();
            debug!("{e:?} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }
}

impl<T, E> LogErrorWithExt<T> for Result<T, E>
where
    E: Debug,
{
    #[track_caller]
    fn log_error_msg(self, message: &str) -> Self {
        if let Err(ref e) = self {
            let location = Location::caller();
            error!("{message}: {e:?} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn log_info_msg(self, message: &str) -> Self {
        if let Err(ref e) = self {
            let location = Location::caller();
            info!("{message}: {e:?} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn log_debug_msg(self, message: &str) -> Self {
        if let Err(ref e) = self {
            let location = Location::caller();
            debug!("{message}: {e:?} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }
}

impl<T, R> LogDerefWithExt<T> for R
where
    R: Deref<Target = Option<T>>,
{
    #[track_caller]
    fn log_deref_error_msg(self, message: &str) -> Self {
        if self.is_none() {
            let location = Location::caller();
            error!("{message} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn log_deref_info_msg(self, message: &str) -> Self {
        if self.is_none() {
            let location = Location::caller();
            info!("{message} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }
}

impl<T> LogOptionWithExt<T> for Option<T> {
    #[track_caller]
    fn log_error_msg(self, message: &str) -> Self {
        if self.is_none() {
            let location = Location::caller();
            error!("{message} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn log_info_msg(self, message: &str) -> Self {
        if self.is_none() {
            let location = Location::caller();
            info!("{message} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn log_debug_msg(self, message: &str) -> Self {
        if self.is_none() {
            let location = Location::caller();
            debug!("{message} (called from {}:{}:{})", location.file(), location.line(), location.column());
        }
        self
    }

    #[track_caller]
    fn report_msg(self, message: &'static str) -> Result<T, Report> {
        if let Some(this) = self {
            Ok(this)
        } else {
            Err(Report::msg(message))
        }
    }
}
