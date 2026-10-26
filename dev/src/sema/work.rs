//! Deterministic semantic demand observations (`SemanticGraph` counters). See NOTICE.
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
}
impl Work {
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
