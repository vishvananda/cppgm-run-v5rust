// Safe Rust storage for analyzer_calls.cpp's query-local argument lists. See NOTICE.
//! A short argument snapshot releases the syntax borrow before semantic demand
//! can append instantiated nodes. Only compact IDs are copied; syntax and facts
//! remain in the unit's slabs. The common case requires no allocation.
use crate::syntax::ast;

pub(super) enum Arguments {
    Empty,
    Inline {
        values: [ast::ExprId; 8],
        count: usize,
    },
    Many(Vec<ast::ExprId>),
}
impl Arguments {
    pub(super) fn new(source: &[ast::ExprId]) -> Self {
        let Some(&first) = source.first() else {
            return Self::Empty;
        };
        if source.len() <= 8 {
            let mut values = [first; 8];
            values[..source.len()].copy_from_slice(source);
            Self::Inline {
                values,
                count: source.len(),
            }
        } else {
            Self::Many(source.to_vec())
        }
    }
    pub(super) fn as_slice(&self) -> &[ast::ExprId] {
        match self {
            Self::Empty => &[],
            Self::Inline { values, count } => &values[..*count],
            Self::Many(values) => values,
        }
    }
}
