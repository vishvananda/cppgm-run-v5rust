//! Telemetry compilation must not execute report-only work when disabled.
use cppgm_rust::support::telemetry::{self, Clock};
use std::cell::Cell;

#[test]
fn observation_closures_follow_the_build_feature() {
    let visits = Cell::new(0);
    let observed = telemetry::observe(|| {
        visits.set(visits.get() + 1);
        7_u64
    });
    assert_eq!(visits.get(), u32::from(telemetry::ENABLED));
    #[cfg(feature = "telemetry")]
    assert_eq!(observed, 7);
    #[cfg(not(feature = "telemetry"))]
    {
        assert_eq!(std::mem::size_of_val(&observed), 0);
        assert_eq!(std::mem::size_of::<Clock>(), 0);
        assert_eq!(Clock::now().elapsed(), std::time::Duration::ZERO);
    }
    let _ = Clock::now();
}
