//! Switch MIR publication and FIFO deferred subtree emission. See NOTICE.
use super::{
    condition::Condition as C,
    condition_mir::FlagOp,
    control_selection::Control,
    mir::{BlockId, BlockName, Instruction, Op},
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    switch_plan::{self, Node, Plans, Table, Tree},
    value_facts::{Alias, normalize},
    x86::Register as R,
};
use crate::lowir::{
    Error,
    literals::Constant,
    model::{BlockId as LowBlock, Value},
    vocabulary::{Binary, Type},
};
fn imm(value: i64) -> Operand {
    Operand::Immediate(i128::from(value).cast_unsigned())
}
fn compare(s: &mut Selection<'_>, ty: Type, left: R, value: i64) {
    s.emit(ScalarOp::Condition(FlagOp::Compare(
        ty,
        Operand::Register(left),
        imm(value),
    )));
}
fn offset(s: &mut Selection<'_>, selector: R, value: i64, last: bool) -> R {
    let r = if selector != R::R10 || last {
        R::R10
    } else {
        R::R11
    };
    if r != selector {
        s.emit(ScalarOp::Move(Type::I64, r, Operand::Register(selector)));
    }
    if value != 0 {
        s.emit(ScalarOp::Binary(Binary::Sub, Type::I64, r, imm(value)));
    }
    r
}
fn masks(s: &mut Selection<'_>, tree: &Tree, r: R) {
    for &(bits, target) in &tree.masks {
        s.emit(ScalarOp::Move(Type::I64, R::R11, imm(bits.cast_signed())));
        s.emit(ScalarOp::Condition(FlagOp::BitTest(
            Type::I64,
            R::R11,
            Operand::Register(r),
        )));
        s.branch(C::Below, target);
    }
}
impl Selection<'_> {
    pub(super) fn clustered_switch(
        &mut self,
        control: &Control,
        from: LowBlock,
        cases: &[(Value, LowBlock)],
        source: R,
        ty: Type,
        fallback: BlockId,
    ) -> Result<bool, Error> {
        let mut values = Vec::with_capacity(cases.len());
        for &(value, to) in cases {
            let Alias::Constant(Constant::Integer(bits)) = self.facts.resolve(self.body, value)
            else {
                return Ok(false);
            };
            values.push((
                super::mir::low_word(normalize(bits, ty))?,
                control.target(from, to)?,
            ));
        }
        if values.is_empty() {
            return Ok(false);
        }
        let (clusters, targets) = switch_plan::clusters(&mut values);
        if let Some(entries) = switch_plan::table_size(&values, targets) {
            let first = self.mir.jump_targets.len();
            let end = first.checked_add(entries).ok_or(Error::Limit)?;
            let _ = u32::try_from(end).map_err(|_| Error::Limit)?;
            let table = Table {
                first: u32::try_from(first).map_err(|_| Error::Limit)?,
                count: u32::try_from(entries).map_err(|_| Error::Limit)?,
            };
            self.mir.jump_targets.resize(end, fallback);
            let low = values[0].0;
            for &(value, target) in values.iter().rev() {
                let index =
                    usize::try_from(value.cast_unsigned().wrapping_sub(low.cast_unsigned()))
                        .map_err(|_| Error::Limit)?;
                self.mir.jump_targets[first + index] = target;
            }
            let r = offset(self, source, low, true);
            compare(
                self,
                Type::I64,
                r,
                i64::try_from(entries - 1).map_err(|_| Error::Limit)?,
            );
            self.branch(C::Above, fallback);
            self.mir.instructions.push(Instruction {
                volatile: false,
                op: Op::JumpTable(table),
                location: self.location,
            });
            self.mir.jump_tables += 1;
            return Ok(true);
        }
        let Some(tree) = switch_plan::tree(
            &clusters,
            source,
            ty,
            fallback,
            self.facts.level,
            self.location,
        ) else {
            return Ok(false);
        };
        let mut plans = std::mem::take(&mut self.switches);
        let index = plans.trees.len();
        let (lo, hi) = switch_plan::bounds(ty);
        let count = tree.tests.len();
        plans.trees.push(tree);
        let result = emit_tests(
            self,
            &mut plans,
            Node {
                tree: index,
                tests: 0..count,
                lo,
                hi,
                outside: false,
                mir: BlockId(0),
            },
        );
        self.switches = plans;
        result.map(|()| true)
    }
    pub(super) fn deferred_switches(&mut self) -> Result<(), Error> {
        let mut plans = std::mem::take(&mut self.switches);
        let mut index = 0;
        while index < plans.nodes.len() {
            let node = plans.nodes[index].clone();
            let created = self.mir.begin_block(BlockName::Switch)?;
            if created != node.mir {
                return Err(Error::Construction(
                    "switch subtree block out of order".into(),
                ));
            }
            self.location = plans.trees[node.tree].location;
            emit_tests(self, &mut plans, node)?;
            self.mir.end_block();
            index += 1;
        }
        Ok(())
    }
}
fn emit_tests(s: &mut Selection<'_>, plans: &mut Plans, mut node: Node) -> Result<(), Error> {
    // Reborrow each tree after publishing a node; no tree or test is cloned.
    while plans.trees[node.tree].binary_search && node.tests.len() >= 4 {
        let t = &plans.trees[node.tree];
        let middle = usize::midpoint(node.tests.start, node.tests.end);
        let pivot = t.tests[middle].lo;
        let (ty, selector, sign) = (t.ty, t.selector, t.sign);
        let target = plans.defer(Node {
            tests: node.tests.start..middle,
            hi: pivot - 1,
            ..node.clone()
        })?;
        compare(s, ty, selector, pivot);
        s.branch(if sign { C::Less } else { C::Below }, target);
        node.tests.start = middle;
        node.lo = pivot;
        // The recursive upper-half call in C++ resets outside_window.
        node.outside = false;
    }
    let t = &plans.trees[node.tree];
    let flags = t.binary_search
        && node.tests.start != 0
        && node.tests.end == t.tests.len()
        && node.lo == t.tests[node.tests.start].lo
        && s.mir.instructions.len() >= 2
        && s.mir
            .instructions
            .last()
            .is_some_and(|i| matches!(i.op, Op::Branch(..)));
    if t.window_first && !node.outside {
        let w = *t.tests[node.tests.clone()]
            .iter()
            .find(|c| c.window)
            .ok_or(Error::Limit)?;
        let r = offset(s, t.selector, w.lo, false);
        let target = plans.defer(Node {
            outside: true,
            ..node
        })?;
        compare(s, Type::I64, r, w.hi - w.lo);
        s.branch(C::Above, target);
        let t = &plans.trees[plans.nodes.last().ok_or(Error::Limit)?.tree];
        masks(s, t, r);
        s.jump(t.fallback);
        return Ok(());
    }
    let mut window = None;
    for i in node.tests.clone() {
        let c = t.tests[i];
        if c.window {
            if !node.outside {
                window = Some(c);
            }
            continue;
        }
        if c.lo <= node.lo && c.hi >= node.hi {
            s.jump(c.target);
            return Ok(());
        }
        if c.lo == c.hi {
            if !(flags && i == node.tests.start) {
                compare(s, t.ty, t.selector, c.lo);
            }
            s.branch(C::Equal, c.target);
        } else if c.lo <= node.lo {
            compare(s, t.ty, t.selector, c.hi);
            s.branch(if t.sign { C::LessEqual } else { C::BelowEqual }, c.target);
        } else if c.hi >= node.hi {
            compare(s, t.ty, t.selector, c.lo);
            s.branch(
                if t.sign {
                    C::GreaterEqual
                } else {
                    C::AboveEqual
                },
                c.target,
            );
        } else {
            let r = offset(s, t.selector, c.lo, false);
            compare(s, Type::I64, r, c.hi - c.lo);
            s.branch(C::BelowEqual, c.target);
        }
    }
    if let Some(w) = window {
        let r = offset(s, t.selector, w.lo, true);
        if !(w.lo <= node.lo && w.hi >= node.hi) {
            compare(s, Type::I64, r, w.hi - w.lo);
            s.branch(C::Above, t.fallback);
        }
        masks(s, t, r);
    }
    s.jump(t.fallback);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    #[test]
    fn deferred_tree_and_table_survive_final_block_relocation() -> Result<(), Error> {
        let mut source = String::new();
        for (name, stride) in [("dense", 1), ("sparse", 100)] {
            write!(source, "function @{name}(%selector : i64) -> i32 {{\n block ^entry:\n switch %selector, ^fallback").unwrap();
            for i in 0..12 {
                write!(source, ", {}:^target{i}", i * stride).unwrap();
            }
            source.push('\n');
            for i in 0..12 {
                write!(source, " block ^target{i}:\n return i32 {i}\n").unwrap();
            }
            source.push_str(" block ^fallback:\n return i32 -1\n}\n");
        }
        let program = crate::lowir::test_program(source.as_bytes())?;
        for (index, function) in program.functions().iter().enumerate() {
            let symbol = program.find_symbol(function.name).ok_or(Error::Limit)?;
            let selected = super::super::value_selection::select(&program, symbol, function, 2)?;
            assert_eq!(selected.jump_tables, usize::from(index == 0));
            let mut view = Vec::new();
            selected.write(&program, &mut view)?;
            let view = String::from_utf8(view).unwrap();
            if index == 1 {
                assert!(view.contains("^switch."));
            }
            let mut image = super::super::image::Image::new(program.symbols().len());
            let emitted = super::super::emission::emit(&selected, &mut image, None)?;
            assert!(emitted.code_bytes > 0);
            // A function-local table needs no object relocation, including
            // after placement and branch shortening changed every label.
            assert_eq!(emitted.fixups, 0);
            for block in 0..selected.blocks.len() {
                super::super::frame_lifetimes::successors(&selected, block, |target| {
                    assert!(target < selected.blocks.len());
                });
            }
        }
        Ok(())
    }
}
