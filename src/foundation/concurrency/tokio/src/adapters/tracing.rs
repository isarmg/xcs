macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "adapters-tracing")]
        tracing::trace!($($arg)*);
    };
}
