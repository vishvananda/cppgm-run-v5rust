//! Interned static handler stacks and monotone joins (`select/host_eh.cpp`). See NOTICE.
use crate::lowir::{Error, model::BlockId};
use crate::support::hash::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub(super) struct State(u32);
impl State {
    fn index(self) -> usize {
        self.0 as usize
    }
    pub(super) fn empty(self) -> bool {
        self.0 == 0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub(super) enum Top {
    #[default]
    None,
    Try,
    Cleanup,
    Mixed,
}
impl Top {
    fn combine(self, other: Self) -> Self {
        if self == Self::None {
            other
        } else if other == Self::None || self == other {
            self
        } else {
            Self::Mixed
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Tail {
    pub(super) lo: u8,
    pub(super) hi: u8,
    pub(super) top: Top,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Key {
    Record(State, BlockId, bool, bool),
    Tail(State, Tail),
}
#[derive(Clone, Copy, Default)]
pub(super) struct Node {
    pub(super) parent: State,
    pub(super) landing: Option<BlockId>,
    depth: u32,
    pub(super) cleanup: bool,
    pub(super) active: bool,
    pub(super) inside: bool,
    pub(super) tail: Option<Tail>,
    pub(super) protect: State,
}
impl Node {
    fn join_position(self, other: Self) -> Result<Option<(BlockId, bool, bool)>, Error> {
        if self.landing != other.landing || self.tail.is_some() || other.tail.is_some() {
            return Ok(None);
        }
        if self.active != other.active {
            let active_cleanup = self.active && self.cleanup || other.active && other.cleanup;
            if !active_cleanup {
                return Err(incompatible());
            }
            return Ok(Some((self.landing.ok_or(Error::Limit)?, true, true)));
        }
        if self.cleanup == other.cleanup {
            return Ok(Some((
                self.landing.ok_or(Error::Limit)?,
                self.cleanup,
                self.active,
            )));
        }
        if !self.active {
            return Err(incompatible());
        }
        Ok(None)
    }
}
pub(super) struct Regions {
    nodes: Vec<Node>,
    index: HashMap<Key, State>,
}
impl Default for Regions {
    fn default() -> Self {
        Self {
            nodes: vec![Node::default()],
            index: HashMap::default(),
        }
    }
}
fn incompatible() -> Error {
    Error::Diagnostic("control-flow paths with different EH regions merge".into())
}
impl Regions {
    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }
    pub(super) fn get(&self, state: State) -> Node {
        self.nodes[state.index()]
    }
    pub(super) fn index(state: State) -> usize {
        state.index()
    }
    fn insert(&mut self, key: Key, node: Node) -> Result<State, Error> {
        let id = State(u32::try_from(self.nodes.len()).map_err(|_| Error::Limit)?);
        self.nodes.push(node);
        self.index.insert(key, id);
        Ok(id)
    }
    pub(super) fn record(
        &mut self,
        parent: State,
        landing: BlockId,
        cleanup: bool,
        active: bool,
    ) -> Result<State, Error> {
        if landing.index() >= 1 << 28 {
            return Err(Error::Diagnostic("too many blocks".into()));
        }
        let key = Key::Record(parent, landing, cleanup, active);
        if let Some(&id) = self.index.get(&key) {
            return Ok(id);
        }
        let below = self.get(parent);
        let id = State(u32::try_from(self.nodes.len()).map_err(|_| Error::Limit)?);
        self.insert(
            key,
            Node {
                parent,
                landing: Some(landing),
                depth: below.depth.checked_add(1).ok_or(Error::Limit)?,
                cleanup,
                active,
                inside: !parent.empty() && (below.active || below.inside),
                tail: None,
                protect: if active { below.protect } else { id },
            },
        )
    }
    pub(super) fn tail(&mut self, base: State, lo: u32, hi: u32, top: Top) -> Result<State, Error> {
        if hi == 0 {
            return Ok(base);
        }
        if hi > 255 {
            return Err(Error::Diagnostic("too many landing records merge".into()));
        }
        let tail = Tail {
            lo: u8::try_from(lo).map_err(|_| Error::Limit)?,
            hi: u8::try_from(hi).map_err(|_| Error::Limit)?,
            top,
        };
        let key = Key::Tail(base, tail);
        if let Some(&id) = self.index.get(&key) {
            return Ok(id);
        }
        let below = self.get(base);
        self.insert(
            key,
            Node {
                parent: base,
                landing: None,
                depth: below.depth.checked_add(1).ok_or(Error::Limit)?,
                active: true,
                inside: true,
                tail: Some(tail),
                protect: below.protect,
                ..Node::default()
            },
        )
    }
    // Position joins are deferred explicitly: no native recursion for nested records.
    pub(super) fn merge(&mut self, mut a: State, mut b: State) -> Result<State, Error> {
        let mut pending = Vec::new();
        let mut result;
        loop {
            if a == b {
                result = a;
                break;
            }
            let mut base = [a, b];
            let mut lo = [0_u32; 2];
            let mut hi = [0_u32; 2];
            let mut top = [Top::None; 2];
            let mut climbed = [0_u32; 2];
            let mut first = [Top::None; 2];
            for side in 0..2 {
                let node = self.get(base[side]);
                if let Some(tail) = node.tail {
                    lo[side] = tail.lo.into();
                    hi[side] = tail.hi.into();
                    top[side] = tail.top;
                    base[side] = node.parent;
                }
            }
            let climb = |side: usize,
                         base: &mut [State; 2],
                         climbed: &mut [u32; 2],
                         first: &mut [Top; 2]|
             -> Result<(), Error> {
                let node = self.get(base[side]);
                if base[side].empty() || !node.active || node.tail.is_some() {
                    return Err(incompatible());
                }
                if climbed[side] == 0 {
                    first[side] = if node.cleanup { Top::Cleanup } else { Top::Try };
                }
                climbed[side] = climbed[side]
                    .checked_add(1)
                    .ok_or_else(|| Error::Diagnostic("too many landing records merge".into()))?;
                base[side] = node.parent;
                Ok(())
            };
            while self.get(base[0]).depth > self.get(base[1]).depth {
                climb(0, &mut base, &mut climbed, &mut first)?;
            }
            while self.get(base[1]).depth > self.get(base[0]).depth {
                climb(1, &mut base, &mut climbed, &mut first)?;
            }
            let mut position = None;
            while base[0] != base[1] {
                let x = self.get(base[0]);
                let y = self.get(base[1]);
                if let Some((landing, cleanup, active)) = x.join_position(y)? {
                    position = Some((x.parent, y.parent, landing, cleanup, active));
                    break;
                }
                climb(0, &mut base, &mut climbed, &mut first)?;
                climb(1, &mut base, &mut climbed, &mut first)?;
            }
            let mut merged_top = Top::None;
            for side in 0..2 {
                let mut t = if hi[side] > 0 { top[side] } else { Top::None };
                if climbed[side] > 0 && lo[side] == 0 {
                    t = t.combine(first[side]);
                }
                merged_top = merged_top.combine(t);
                lo[side] = lo[side].checked_add(climbed[side]).ok_or(Error::Limit)?;
                hi[side] = hi[side].checked_add(climbed[side]).ok_or(Error::Limit)?;
            }
            let tail = (lo[0].min(lo[1]), hi[0].max(hi[1]), merged_top);
            if let Some((x, y, landing, cleanup, active)) = position {
                pending.push((landing, cleanup, active, tail));
                a = x;
                b = y;
            } else {
                result = self.tail(base[0], tail.0, tail.1, tail.2)?;
                break;
            }
        }
        while let Some((landing, cleanup, active, (lo, hi, top))) = pending.pop() {
            let base = self.record(result, landing, cleanup, active)?;
            result = self.tail(base, lo, hi, top)?;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{InstructionKind as I, Program},
        vocabulary::Type,
    };

    #[test]
    fn joins_preserve_protecting_records_and_summarize_only_active_suffixes() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void)?;
        let outer = builder.block_id("outer")?;
        let left = builder.block_id("left")?;
        let right = builder.block_id("right")?;
        builder.block("entry")?;
        builder.effect(I::Return(Type::Void, None))?;
        builder.finish()?;
        let mut regions = Regions::default();
        let root = State::default();
        let protect = regions.record(root, outer, false, false)?;
        assert_eq!(protect, regions.record(root, outer, false, false)?);
        let a = regions.record(protect, left, true, true)?;
        let b = regions.record(protect, right, true, true)?;
        let merged = regions.merge(a, b)?;
        let reverse = regions.merge(b, a)?;
        assert_eq!(merged, reverse);
        assert_eq!(regions.get(merged).protect, protect);
        assert_eq!(
            regions.get(merged).tail,
            Some(Tail {
                lo: 1,
                hi: 1,
                top: Top::Cleanup
            })
        );
        assert_eq!(regions.merge(merged, a)?, merged);
        let wide = regions.merge(protect, merged)?;
        assert_eq!(
            regions.get(wide).tail,
            Some(Tail {
                lo: 0,
                hi: 1,
                top: Top::Cleanup
            })
        );
        let inactive = regions.record(protect, right, true, false)?;
        assert!(regions.merge(a, inactive).is_err());
        // A no-match forward into the same landing agrees on the pop, even
        // when one predecessor already marked its try record as cleanup.
        let active = regions.record(protect, right, true, true)?;
        let forwarded = regions.merge(inactive, active)?;
        assert_eq!(forwarded, active);
        assert_eq!(regions.get(forwarded).protect, protect);
        Ok(())
    }

    #[test]
    fn deep_position_joins_use_an_explicit_continuation_stack() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::Void)?;
        let landing = builder.block_id("landing")?;
        builder.block("entry")?;
        builder.effect(I::Return(Type::Void, None))?;
        builder.finish()?;
        let mut regions = Regions::default();
        let root = State::default();
        let mut a = regions.record(root, landing, false, false)?;
        let mut b = regions.record(root, landing, true, true)?;
        for _ in 0..4096 {
            a = regions.record(a, landing, true, true)?;
            b = regions.record(b, landing, true, true)?;
        }
        let merged = regions.merge(a, b)?;
        assert!(regions.get(merged).protect.empty());
        assert_eq!(regions.merge(merged, a)?, merged);
        Ok(())
    }
}
