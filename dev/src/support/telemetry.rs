//! Optional observations (`support/telemetry.h`); never semantic state.

/// Whether counters, phase clocks and statistics reports are compiled in.
pub const ENABLED: bool = cfg!(feature = "telemetry");
/// Frozen diagnostic for a statistics option in a build without observations.
pub const UNAVAILABLE: &str = "statistics are unavailable in the telemetry-off build";

#[cfg(feature = "telemetry")]
pub type Counter<T> = T;
#[cfg(not(feature = "telemetry"))]
pub use disabled::Counter;

/// Initialize an observation without introducing a value-reading off-build API.
#[must_use]
pub fn counter<T>(value: T) -> Counter<T> {
    #[cfg(feature = "telemetry")]
    {
        value
    }
    #[cfg(not(feature = "telemetry"))]
    {
        drop(value);
        Counter::default()
    }
}

/// Observe an existing result only when observations are compiled in.
#[must_use]
pub fn observe<T>(read: impl FnOnce() -> T) -> Counter<T> {
    #[cfg(feature = "telemetry")]
    {
        read()
    }
    #[cfg(not(feature = "telemetry"))]
    {
        let _ = read;
        Counter::default()
    }
}

#[cfg(feature = "telemetry")]
pub(crate) type Set<T> = crate::support::hash::HashSet<T>;
#[cfg(not(feature = "telemetry"))]
pub(crate) use disabled::Set;

/// A phase clock whose disabled representation retains no clock or storage.
#[derive(Clone, Copy)]
pub struct Clock {
    #[cfg(feature = "telemetry")]
    start: std::time::Instant,
}
impl Clock {
    #[must_use]
    pub fn now() -> Self {
        Self {
            #[cfg(feature = "telemetry")]
            start: std::time::Instant::now(),
        }
    }
    #[must_use]
    pub fn elapsed(&self) -> std::time::Duration {
        #[cfg(feature = "telemetry")]
        {
            self.start.elapsed()
        }
        #[cfg(not(feature = "telemetry"))]
        {
            std::time::Duration::ZERO
        }
    }
}

#[cfg(not(feature = "telemetry"))]
mod disabled {
    use std::{
        marker::PhantomData,
        ops::{Add, AddAssign, SubAssign},
    };
    /// An observation with no value. It intentionally implements no numeric
    /// conversion, comparison or formatting: such a read is not telemetry.
    pub struct Counter<T>(PhantomData<T>);
    pub(crate) struct Set<T>(PhantomData<T>);
    impl<T> Default for Set<T> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }
    impl<T> Set<T> {
        pub(crate) fn insert(&mut self, _: T) {
            let _ = self;
        }
    }
    impl<T> Copy for Counter<T> {}
    impl<T> Clone for Counter<T> {
        fn clone(&self) -> Self {
            *self
        }
    }
    impl<T> Default for Counter<T> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }
    impl<T> AddAssign<T> for Counter<T> {
        fn add_assign(&mut self, _: T) {}
    }
    impl<T> AddAssign<Self> for Counter<T> {
        fn add_assign(&mut self, _: Self) {}
    }
    impl<T> SubAssign<T> for Counter<T> {
        fn sub_assign(&mut self, _: T) {}
    }
    impl<T> Add<T> for Counter<T> {
        type Output = Self;
        fn add(self, _: T) -> Self {
            self
        }
    }
    impl<T> Add<Self> for Counter<T> {
        type Output = Self;
        fn add(self, _: Self) -> Self {
            self
        }
    }
    impl<T> Counter<T> {
        #[must_use]
        pub fn max(self, _: T) -> Self {
            self
        }
        #[must_use]
        pub fn wrapping_add(self, _: T) -> Self {
            self
        }
        #[must_use]
        pub fn checked_add<U>(self, _: U) -> Option<Self> {
            Some(self)
        }
    }
}
