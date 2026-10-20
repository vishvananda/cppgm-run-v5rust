//! Stable parallel moves into the bounded scalar ABI register set. See NOTICE.
use super::{
    memory::Base,
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    x86::Register as R,
};
use crate::lowir::{Error, vocabulary::Type};
#[derive(Clone, Copy)]
pub(super) struct Transfer {
    pub(super) destination: R,
    pub(super) source: Operand,
    pub(super) ty: Type,
    pub(super) source_type: Option<Type>,
    pub(super) address: bool,
}
impl Transfer {
    pub(super) fn emit(self, selection: &mut Selection<'_>) {
        if self.address {
            selection.emit(ScalarOp::Lea(self.destination, self.source));
        } else {
            selection.load(
                self.destination,
                self.source,
                self.source_type.unwrap_or(self.ty),
            );
        }
    }
    fn redirect(&mut self, from: R, to: R) {
        match &mut self.source {
            Operand::Register(r) if *r == from => *r = to,
            Operand::Memory(m) => {
                if matches!(m.base,Base::Register(r) if r==from) {
                    m.base = Base::Register(to);
                }
                if m.index == Some(from) {
                    m.index = Some(to);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn emit(selection: &mut Selection<'_>, moves: &mut Vec<Transfer>) -> Result<(), Error> {
    moves.retain(|m| m.address || m.source.register() != Some(m.destination));
    // Register destinations are unique. Calls have at most six GPR arguments,
    // the variadic count and a protected indirect target: a bounded scan.
    let mut destinations = 0_u32;
    for m in moves.iter() {
        let bit = 1 << m.destination.code();
        if destinations & bit != 0 {
            return Err(Error::Construction(
                "duplicate parallel register destination".into(),
            ));
        }
        destinations |= bit;
    }
    while !moves.is_empty() {
        let ready = moves.iter().enumerate().position(|(i, m)| {
            moves
                .iter()
                .enumerate()
                .all(|(j, n)| i == j || n.source.mask() & (1 << m.destination.code()) == 0)
        });
        if let Some(i) = ready {
            moves.remove(i).emit(selection);
            continue;
        }
        let first = moves[0];
        selection.load(R::R10, Operand::Register(first.destination), first.ty);
        for m in moves.iter_mut() {
            m.redirect(first.destination, R::R10);
        }
    }
    Ok(())
}
