pub mod codec;
pub mod coff;
pub mod commands;
pub mod config;
pub mod protocol;
pub mod proxy;
pub mod runtime;
pub mod transport;

#[cfg(feature = "diagnostics")]
#[macro_export]
macro_rules! diagnostic {
    ($($arg:tt)*) => {{ eprintln!($($arg)*) }};
}

#[cfg(not(feature = "diagnostics"))]
#[macro_export]
macro_rules! diagnostic {
    ($($arg:tt)*) => {{}};
}
