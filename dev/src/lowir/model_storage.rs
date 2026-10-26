//! Reserved typed payload buffers; lookup indexes and allocator overhead are excluded. See NOTICE.
use super::{Body, GlobalInit, InstructionKind, Program};
fn reserved<T>(values: &Vec<T>) -> usize {
    values.capacity() * std::mem::size_of::<T>()
}
impl Program {
    pub(crate) fn storage_bytes(&self) -> usize {
        self.names.storage_bytes()
            + reserved(&self.names.text)
            + reserved(&self.names.logical)
            + reserved(&self.symbols)
            + reserved(&self.functions)
            + reserved(&self.signatures)
            + reserved(&self.globals)
            + reserved(&self.aliases)
            + self
                .signatures
                .iter()
                .map(|signature| reserved(&signature.parameters))
                .sum::<usize>()
            + self
                .globals
                .iter()
                .map(|global| match &global.init {
                    GlobalInit::Data(items) => reserved(items),
                    _ => 0,
                })
                .sum::<usize>()
            + self
                .functions
                .iter()
                .filter_map(|function| function.body.as_ref())
                .map(Body::storage_bytes)
                .sum::<usize>()
    }
}
impl Body {
    fn storage_bytes(&self) -> usize {
        reserved(&self.literals)
            + reserved(&self.operands)
            + reserved(&self.temps)
            + reserved(&self.slots)
            + reserved(&self.slot_names)
            + reserved(&self.blocks)
            + reserved(&self.instructions)
            + reserved(&self.locations)
            + reserved(&self.layout)
            + self
                .blocks
                .iter()
                .map(|block| reserved(&block.instructions))
                .sum::<usize>()
            + self
                .instructions
                .iter()
                .map(|instruction| match &instruction.kind {
                    InstructionKind::Phi(_, pairs) => reserved(pairs),
                    InstructionKind::Switch(_, _, pairs) => reserved(pairs),
                    _ => 0,
                })
                .sum::<usize>()
    }
}
