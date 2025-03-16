#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

mod logging;
mod panic_handler;
pub use backtrace::Backtrace;
pub use color_eyre::{Context, Report, Result, eyre::WrapErr};
pub use log::{LevelFilter, debug, error, info};
pub use logging::{
    LogDerefWithExt, LogErrorExt, LogErrorWithExt, LogOptionWithExt, MakeReportExt, WORKSPACE_CRATES,
    initialize_logging,
};
pub use minitrace;
pub use panic_handler::initialize_panic_handler;
use std::{
    error::Error as StdError,
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::Deref,
};
pub use strip_ansi_escapes::strip_str;

pub type TypedResult<R, E> = Result<R, TypedReport<E>>;

pub struct TypedReport<E> {
    pub report: Report,
    phantom: PhantomData<E>,
}

impl<E> From<TypedReport<E>> for Report {
    fn from(value: TypedReport<E>) -> Self {
        value.report
    }
}

impl<E: Send + Sync + std::error::Error + 'static> TypedReport<E> {
    #[must_use]
    pub fn into_inner(self) -> E {
        self.report.downcast().unwrap()
    }

    #[must_use]
    pub fn inner(&self) -> &E {
        self.report.downcast_ref().unwrap()
    }

    pub fn new(error: E) -> Self {
        Self { report: Report::new(error), phantom: PhantomData }
    }
}

impl<E> From<E> for TypedReport<E>
where
    E: StdError + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        Self::new(value)
    }
}

impl<E: Send + Sync + std::error::Error + 'static> Deref for TypedReport<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

pub trait AsReport<R> {
    fn inner_result(self) -> Result<R>;
}

impl<R, E> AsReport<R> for Result<R, TypedReport<E>>
where
    E: Send + Sync + std::error::Error + 'static,
{
    fn inner_result(self) -> Result<R> {
        self.map_err(|e| e.report)
    }
}

impl<E> Debug for TypedReport<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.report, f)
    }
}

impl<E> Display for TypedReport<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.report, f)
    }
}

pub trait ColorLess {
    fn no_color_debug(&self) -> String;
    fn no_color_display(&self) -> String;
}

impl ColorLess for Report {
    fn no_color_debug(&self) -> String {
        strip_str(format!("{self:?}"))
    }

    fn no_color_display(&self) -> String {
        strip_str(format!("{self}"))
    }
}

#[macro_export]
macro_rules! error_backtrace {
    ($($arg:tt)*) => {{
        let backtrace =  errors::Backtrace::new();
        errors::info!("{}: {:?}", format_args!($($arg)*), backtrace);
    }};
}

/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! trace_dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {{
        match $ex {
            value => {
                tracing::event!(target: $target, $level, ?value, stringify!($ex));
                value
            }
        }
    }};
    (level: $level:expr, $ex:expr) => {
        trace_dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        trace_dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        trace_dbg!(level: tracing::Level::DEBUG, $ex)
    };
}
