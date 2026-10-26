//! Deterministic semantic demand observations (`SemanticGraph` counters). See NOTICE.
use std::cell::Cell;
use std::io::Write;

#[derive(Clone, Copy, Default)]
pub(crate) struct Work {
    pub(crate) specializations: usize,
    pub(crate) instantiated_classes: usize,
    pub(crate) instantiated_functions: usize,
    pub(crate) instantiated_variables: usize,
    pub(crate) constant_evaluations: usize,
    pub(crate) resolutions: usize,
    pub(crate) candidates: usize,
    pub(crate) virtual_slots: usize,
    pub(crate) override_steps: usize,
    pub(crate) instantiated_arguments: usize,
    pub(crate) value_substitutions: usize,
    pub(crate) partial_candidates: usize,
    pub(crate) substitution_failures: usize,
    pub(crate) limited_lookups: usize,
    pub(crate) constant_steps: usize,
    pub(crate) base_steps: usize,
}
#[derive(Default)]
pub(super) struct Observations {
    pub(super) totals: Work,
    base_steps: Cell<usize>,
    limited_lookups: Cell<usize>,
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
    pub(crate) fn add(&mut self, other: Self) {
        for (counter, increment) in [
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
    pub(crate) fn write(self, out: &mut impl Write) -> std::io::Result<()> {
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
