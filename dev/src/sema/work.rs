//! Deterministic semantic demand observations (`SemanticGraph` counters). See NOTICE.
use std::cell::Cell;
#[cfg(feature = "telemetry")]
use std::io::Write;

#[derive(Clone, Copy, Default)]
pub struct Work {
    pub expressions: crate::support::telemetry::Counter<usize>,
    pub conversions: crate::support::telemetry::Counter<usize>,
    pub demands: crate::support::telemetry::Counter<usize>,
    pub specializations: crate::support::telemetry::Counter<usize>,
    pub instantiated_classes: crate::support::telemetry::Counter<usize>,
    pub instantiated_functions: crate::support::telemetry::Counter<usize>,
    pub instantiated_variables: crate::support::telemetry::Counter<usize>,
    pub constant_evaluations: crate::support::telemetry::Counter<usize>,
    pub resolutions: crate::support::telemetry::Counter<usize>,
    pub candidates: crate::support::telemetry::Counter<usize>,
    pub virtual_slots: crate::support::telemetry::Counter<usize>,
    pub override_steps: crate::support::telemetry::Counter<usize>,
    pub instantiated_arguments: crate::support::telemetry::Counter<usize>,
    pub value_substitutions: crate::support::telemetry::Counter<usize>,
    pub partial_candidates: crate::support::telemetry::Counter<usize>,
    pub substitution_failures: crate::support::telemetry::Counter<usize>,
    pub limited_lookups: crate::support::telemetry::Counter<usize>,
    pub constant_steps: crate::support::telemetry::Counter<usize>,
    pub constant_call_builds: crate::support::telemetry::Counter<usize>,
    pub constant_call_reuses: crate::support::telemetry::Counter<usize>,
    pub function_order_builds: crate::support::telemetry::Counter<usize>,
    pub function_order_reuses: crate::support::telemetry::Counter<usize>,
    pub base_steps: crate::support::telemetry::Counter<usize>,
}
#[derive(Default)]
pub(super) struct Observations {
    pub(super) totals: Work,
    base_steps: Cell<crate::support::telemetry::Counter<usize>>,
    limited_lookups: Cell<crate::support::telemetry::Counter<usize>>,
}
impl Observations {
    pub(super) fn base_steps(&self, steps: usize) {
        self.base_steps.set(self.base_steps.get() + steps);
    }
    pub(super) fn limited_lookup(&self) {
        self.limited_lookups.set(self.limited_lookups.get() + 1);
    }
    pub(super) fn snapshot(&self) -> Work {
        Work {
            base_steps: self.base_steps.get() + self.totals.base_steps,
            limited_lookups: self.limited_lookups.get(),
            ..self.totals
        }
    }
}
impl Work {
    pub fn add(&mut self, other: Self) {
        for (counter, increment) in [
            (&mut self.expressions, other.expressions),
            (&mut self.conversions, other.conversions),
            (&mut self.demands, other.demands),
            (&mut self.specializations, other.specializations),
            (&mut self.instantiated_classes, other.instantiated_classes),
            (
                &mut self.instantiated_functions,
                other.instantiated_functions,
            ),
            (
                &mut self.instantiated_variables,
                other.instantiated_variables,
            ),
            (&mut self.constant_evaluations, other.constant_evaluations),
            (&mut self.constant_call_builds, other.constant_call_builds),
            (&mut self.constant_call_reuses, other.constant_call_reuses),
            (&mut self.function_order_builds, other.function_order_builds),
            (&mut self.function_order_reuses, other.function_order_reuses),
            (&mut self.resolutions, other.resolutions),
            (&mut self.candidates, other.candidates),
            (&mut self.virtual_slots, other.virtual_slots),
            (&mut self.override_steps, other.override_steps),
            (
                &mut self.instantiated_arguments,
                other.instantiated_arguments,
            ),
            (&mut self.value_substitutions, other.value_substitutions),
            (&mut self.partial_candidates, other.partial_candidates),
            (&mut self.substitution_failures, other.substitution_failures),
            (&mut self.limited_lookups, other.limited_lookups),
            (&mut self.constant_steps, other.constant_steps),
            (&mut self.base_steps, other.base_steps),
        ] {
            *counter += increment;
        }
    }
    #[cfg(feature = "telemetry")]
    pub fn write(self, out: &mut impl Write) -> std::io::Result<()> {
        write!(
            out,
            " specializations={} instantiated_classes={} instantiated_functions={} instantiated_variables={} constant_evaluations={} resolutions={} candidates={}",
            self.specializations,
            self.instantiated_classes,
            self.instantiated_functions,
            self.instantiated_variables,
            self.constant_evaluations,
            self.resolutions,
            self.candidates
        )
    }
}
