use crate::{Report, Result};
use build::{LOG_ENV, LOG_FILE, get_data_dir};

use env_logger::Builder;
use log::{LevelFilter, debug, error, info};
use std::{
    fmt::{Debug, Write as _},
    fs::File,
    io::Write,
    ops::Deref,
    panic::Location,
    sync::LazyLock,
};
use sync::Mutex;

pub static WORKSPACE_CRATES: &str = env!("_WORKSPACE_CRATES");

// SAFETY: this only protects against accidental parallel calls to initialize_logging.
// if set_env is called from another thread for any other reason ( 3rd party etc. ), a data race
// may still occur
static ENV_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub fn initialize_logging() -> Result<()> {
    let directory = get_data_dir();
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(LOG_FILE.clone());

    let log_file = Box::new(Mutex::new(File::create(log_path)?));
    let log_file = Box::leak(log_file);

    let _guard = ENV_MUTEX.lock();

    unsafe {
        let mut rust_log = std::env::var("RUST_LOG").or_else(|_| std::env::var(LOG_ENV.clone())).unwrap_or_else(|_| {
            WORKSPACE_CRATES
                .split(',')
                .map(|crate_name| format!("{}=info", crate_name.replace('-', "_")))
                .collect::<Vec<String>>()
                .join(",")
        });

        // taken from https://github.com/emilk/egui/blob/d811940dcc59a0d863e30d86e35bd41df0d9dee9/crates/egui_demo_app/src/main.rs#L26
        // Silence wgpu log spam (https://github.com/gfx-rs/wgpu/issues/3206)
        for loud_crate in ["naga", "wgpu_core", "wgpu_hal"] {
            if !rust_log.contains(&format!("{loud_crate}=")) {
                let _ = write!(rust_log, ",{loud_crate}=warn");
            }
        }

        std::env::set_var("RUST_LOG", rust_log);
    }

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
        if let Some(this) = self { Ok(this) } else { Err(Report::msg(message)) }
    }
}
