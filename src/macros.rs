macro_rules! debug {
    ($scope:expr, $($arg:tt)+) => {
        #[cfg(feature = "tracing")]
        tracing::debug!(target: concat!("rmq::", $scope), $($arg)+)
    }
}

macro_rules! trace {
    ($scope:expr, $($arg:tt)+) => {
        #[cfg(feature = "tracing")]
        tracing::trace!(target: concat!("rmq::", $scope), $($arg)+)
    }
}
