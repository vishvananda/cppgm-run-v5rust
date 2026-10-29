// Translated from lower/global_lowering.cpp's static data, string_object and constexpr_image. See NOTICE.
//! Typed images become ordered `LowIR` items; unit-local caches share data without sharing local storage.
use super::{Error, symbols::Symbols, types};
use crate::support::hash::HashMap;
use crate::{
    lex::PostTokenKind,
    lowir::{
        literals::{Constant, FloatKind},
        model::{DataItem, Global, GlobalInit, Metadata, Program, SymbolId},
        vocabulary::{Binding, Storage},
    },
    sema::{
        Graph,
        lowering::{StaticItem, StaticValue},
    },
    syntax::ast,
};
/// Static addresses may demand a template object's definition immediately.
/// The unit initialization context follows namespace and function images;
/// isolated generated bodies leave demand in their parent-owned queue.
pub(super) trait Output {
    fn program(&mut self) -> &mut Program;
    fn referenced_variable(
        &mut self,
        cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    ) -> Result<(), Error>;
}
impl Output for Program {
    fn program(&mut self) -> &mut Program {
        self
    }
    fn referenced_variable(
        &mut self,
        _cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
#[derive(Default)]
pub(super) struct Objects {
    pub(super) nulls: super::null_initialization::Layouts,
    strings: HashMap<(crate::lowir::vocabulary::Type, Vec<u64>), SymbolId>,
    string_units: Vec<u64>,
    images: HashMap<Vec<DataItem>, SymbolId>,
    reference_temporaries: HashMap<crate::sema::lowering::EntityId, SymbolId>,
    constant_temporaries: HashMap<crate::sema::lowering::TemporaryId, SymbolId>,
    defined_references: crate::support::hash::HashSet<crate::sema::lowering::EntityId>,
}
impl Objects {
    pub(super) fn take_string_units(&mut self) -> Vec<u64> {
        let mut units = std::mem::take(&mut self.string_units);
        units.clear();
        units
    }
    pub(super) fn reference_defined(&self, entity: crate::sema::lowering::EntityId) -> bool {
        self.defined_references.contains(&entity)
    }
    pub(super) fn define_reference(&mut self, entity: crate::sema::lowering::EntityId) {
        self.defined_references.insert(entity);
    }
}
fn integer(bits: u64) -> Constant {
    Constant::Integer(u128::from_le_bytes(
        i128::from(i64::from_le_bytes(bits.to_le_bytes())).to_le_bytes(),
    ))
}
pub(super) fn soft_item(value: StaticValue) -> Option<DataItem> {
    let StaticValue::Floating(ty, bits) = value else {
        return None;
    };
    let ty = match ty.floating_representation() {
        crate::types::FundamentalType::Float16 | crate::types::FundamentalType::BFloat16 => {
            crate::lowir::vocabulary::Type::U16
        }
        crate::types::FundamentalType::Float128 => crate::lowir::vocabulary::Type::I128,
        _ => return None,
    };
    Some(DataItem::Constant(ty, Constant::Integer(bits)))
}
pub(super) fn constant(value: StaticValue) -> Result<Constant, Error> {
    Ok(match value {
        StaticValue::Integer(bits) => integer(bits),
        StaticValue::WideInteger(bits) => Constant::Integer(bits),
        StaticValue::Floating(ty, bits) if crate::types::interchange::soft(ty) => {
            Constant::Integer(bits)
        }
        StaticValue::Floating(ty, bits) => Constant::Float(
            match ty.floating_representation() {
                crate::types::FundamentalType::Float => FloatKind::F32,
                crate::types::FundamentalType::Double => FloatKind::F64,
                _ => FloatKind::F80,
            },
            bits,
        ),
        _ => {
            return Err(Error::Semantic(crate::sema::Error::Invariant(
                "address used as scalar data",
            )));
        }
    })
}
pub(super) fn address(
    output: &mut (impl Output + ?Sized),
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    value: StaticValue,
) -> Result<(SymbolId, i64), Error> {
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .check_stack()?;
    match value {
        StaticValue::TypeInformation(ty, offset) => Ok((
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .type_information(output.program(), ty)?,
            offset,
        )),
        StaticValue::Temporary(id, offset) => Ok((constant_temporary(output, cx, id)?, offset)),
        StaticValue::Address(entity, offset) => {
            let symbol = if cx.graph.lower_is_variable(entity)
                && cx
                    .graph
                    .lower_variable(entity)?
                    .reference_temporary
                    .is_some()
            {
                reference_temporary(output, cx, entity, true)?
            } else if cx.graph.lower_is_variable(entity) {
                let symbol = cx
                    .symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .variable_symbol(output.program(), entity)?;
                output.referenced_variable(cx)?;
                symbol
            } else {
                cx.symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .demand(entity);
                cx.symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .function_symbol(output.program(), entity)?
            };
            Ok((symbol, offset))
        }
        StaticValue::Literal(expr, offset) => Ok((
            literal_object(
                output.program(),
                &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
                &*cx.graph,
                &*cx.tree,
                expr,
            )?,
            offset,
        )),
        _ => Err(Error::Semantic(crate::sema::Error::Invariant(
            "scalar used as address data",
        ))),
    }
}
fn constant_temporary(
    output: &mut (impl Output + ?Sized),
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    id: crate::sema::lowering::TemporaryId,
) -> Result<SymbolId, Error> {
    if let Some(&symbol) = cx.symbols.data.constant_temporaries.get(&id) {
        return Ok(symbol);
    }
    let (ty, image) = cx.graph.lower_constant_temporary(id)?;
    let aggregate = matches!(
        cx.graph.lower_type(ty)?,
        crate::sema::lowering::TypeView::Class(_) | crate::sema::lowering::TypeView::Array { .. }
    );
    let image = image.to_vec();
    let mut data = items(output, cx, &image)?;
    preserve_static_offsets(&mut data)?;
    let storage = if aggregate {
        super::globals::structured_type(
            &*cx.graph,
            ty,
            &data,
            cx.symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .object_mode(),
        )?
    } else if matches!(cx.graph.lower_type(ty)?, crate::sema::lowering::TypeView::Scalar(
        crate::sema::lowering::Scalar::Basic(kind)) if crate::types::interchange::soft(kind))
        && let [DataItem::Constant(stored, _)] = data.as_slice()
    {
        // constant_global states a held soft value using its encoded item type.
        Some(*stored)
    } else {
        Some(types::storage(&*cx.graph, ty)?)
    };
    let init = if aggregate {
        GlobalInit::Data(data)
    } else {
        match data.as_slice() {
            [DataItem::Constant(_, value)] => GlobalInit::Constant(*value),
            [DataItem::Address(_, symbol, offset)] => GlobalInit::Address(*symbol, *offset),
            [DataItem::Zero(_)] => GlobalInit::Constant(Constant::Integer(0)),
            _ => {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "scalar temporary image shape",
                )));
            }
        }
    };
    let (symbol, object) = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .constant_temporary_symbol(output.program())?;
    let name = output.program().symbol(symbol).name;
    output.program().add_global(
        symbol,
        Global {
            name,
            ty: storage,
            metadata: Metadata {
                binding: Some(Binding::Internal),
                storage: cx
                    .symbols
                    .view(&*cx.graph, &*cx.tree, cx.cursor)
                    .readonly_object(ty)?
                    .then_some(Storage::ReadOnly),
                object,
                ..Metadata::default()
            },
            init,
        },
    )?;
    cx.symbols.data.constant_temporaries.insert(id, symbol);
    Ok(symbol)
}
pub(super) fn items(
    output: &mut (impl Output + ?Sized),
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    image: &[StaticItem],
) -> Result<Vec<DataItem>, Error> {
    let mut out = Vec::with_capacity(image.len());
    for &item in image {
        if let StaticItem::Scalar(_, value @ StaticValue::MemberPointer(_)) = item {
            append_member_items(
                output,
                &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
                &*cx.graph,
                value,
                &mut out,
            )?;
            continue;
        }
        out.push(match item {
            StaticItem::Table(class, segment) => {
                let (symbol, addend) =
                    cx.table_address(output.program(), class, segment.index())?;
                DataItem::Address(
                    crate::lowir::vocabulary::Type::Ptr,
                    symbol,
                    i64::try_from(addend).map_err(|_| crate::lowir::Error::Limit)?,
                )
            }
            StaticItem::BitUnit(bytes, bits) => DataItem::Constant(
                match bytes {
                    1 => crate::lowir::vocabulary::Type::U8,
                    2 => crate::lowir::vocabulary::Type::U16,
                    4 => crate::lowir::vocabulary::Type::U32,
                    _ => crate::lowir::vocabulary::Type::I64,
                },
                integer(bits),
            ),
            StaticItem::Zero(bytes) => {
                DataItem::Zero(i64::try_from(bytes).map_err(|_| crate::lowir::Error::Limit)?)
            }
            StaticItem::Scalar(ty, value) => match value {
                StaticValue::Address(..)
                | StaticValue::Literal(..)
                | StaticValue::Temporary(..)
                | StaticValue::TypeInformation(..) => {
                    let (symbol, offset) = address(output, cx, value)?;
                    DataItem::Address(types::storage(&*cx.graph, ty)?, symbol, offset)
                }
                _ => {
                    if let Some(item) = soft_item(value) {
                        item
                    } else {
                        DataItem::Constant(types::storage(&*cx.graph, ty)?, constant(value)?)
                    }
                }
            },
        });
    }
    Ok(out)
}
/// Layout builders have already supplied padding. Prevent `LowIR`'s natural
/// item alignment from moving an unaligned scalar at publication time.
pub(super) fn preserve_static_offsets(items: &mut Vec<DataItem>) -> Result<(), Error> {
    use crate::lowir::vocabulary::Type;
    fn layout(item: DataItem) -> (u64, u64) {
        match item {
            DataItem::Zero(bytes) => (bytes.cast_unsigned(), 1),
            DataItem::Address(..) | DataItem::Constant(Type::Ptr, _) => (8, 8),
            DataItem::Constant(Type::Object { bytes, align }, _) => {
                (bytes, 1_u64.wrapping_shl(u32::from(align)))
            }
            DataItem::Constant(ty, _) => {
                let size = if ty.bits() != 0 {
                    u64::from(ty.bits().max(8) / 8)
                } else {
                    match ty.float_rank() {
                        1 => 4,
                        2 => 8,
                        _ => 16,
                    }
                };
                (size, size)
            }
        }
    }
    let mut offset = 0_u64;
    let first = items.iter().position(|&item| {
        let (size, align) = layout(item);
        if matches!(item, DataItem::Constant(..)) && offset & (align - 1) != 0 {
            true
        } else {
            offset = offset.wrapping_add(size);
            false
        }
    });
    let Some(first) = first else {
        return Ok(());
    };
    for item in items.split_off(first) {
        let (size, align) = layout(item);
        if let DataItem::Constant(_, Constant::Integer(bits) | Constant::Float(_, bits)) = item
            && offset & (align - 1) != 0
        {
            items
                .try_reserve(usize::try_from(size).map_err(|_| crate::lowir::Error::Limit)?)
                .map_err(|_| crate::lowir::Error::Limit)?;
            let bytes = bits.to_le_bytes();
            for index in 0..size {
                let byte = usize::try_from(index).ok().and_then(|i| bytes.get(i));
                items.push(DataItem::Constant(
                    Type::U8,
                    Constant::Integer(u128::from(byte.copied().unwrap_or(0))),
                ));
            }
        } else {
            items.push(item);
        }
        offset = offset.wrapping_add(size);
    }
    Ok(())
}
/// Member functions use two i64 words, never an i128 item with 16-byte alignment.
pub(super) fn member_items(
    output: &mut (impl Output + ?Sized),
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    value: StaticValue,
) -> Result<Vec<DataItem>, Error> {
    let mut items = Vec::with_capacity(2);
    append_member_items(output, symbols, graph, value, &mut items)?;
    Ok(items)
}
fn append_member_items(
    output: &mut (impl Output + ?Sized),
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    value: StaticValue,
    out: &mut Vec<DataItem>,
) -> Result<(), Error> {
    use crate::lowir::vocabulary::Type;
    let StaticValue::MemberPointer(value) = value else {
        return Err(crate::sema::Error::Invariant("member static image value").into());
    };
    let (_, element) = graph
        .lower_member_pointer_type(value.ty)
        .ok_or(crate::sema::Error::Invariant("member static image type"))?;
    let function = matches!(
        graph.lower_type(element)?,
        crate::sema::lowering::TypeView::Function { .. }
    );
    let scalar = |bits| DataItem::Constant(Type::I64, integer(bits));
    let Some(member) = value.member else {
        out.push(scalar(if function { 0 } else { u64::MAX }));
        if function {
            out.push(scalar(0));
        }
        return Ok(());
    };
    if !function {
        let offset = graph
            .lower_member_offset(member)?
            .wrapping_add(graph.lower_anonymous_member_offset(member, None)?)
            .wrapping_add(value.adjustment.cast_unsigned());
        out.push(scalar(offset));
        return Ok(());
    }
    let low = if let Some(slot) = graph.lower_virtual_slot(member) {
        scalar(slot.byte_offset().wrapping_add(1))
    } else {
        symbols.demand(member);
        DataItem::Address(
            Type::Ptr,
            symbols.function_symbol(output.program(), member)?,
            0,
        )
    };
    out.extend([low, scalar(value.adjustment.cast_unsigned())]);
    Ok(())
}
pub(super) fn literal_object(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    tree: &ast::Tree,
    expr: ast::ExprId,
) -> Result<SymbolId, Error> {
    let ast::ExprKind::Literal(literal) = tree.expressions.get(expr).kind else {
        return Err(crate::sema::Error::Invariant("string address without literal").into());
    };
    if let Some(crate::sema::lowering::LiteralCall::Raw { digits, .. }) =
        graph.lower_literal_call(expr)?
    {
        let mut units = symbols.data.take_string_units();
        let digits = usize::try_from(digits).map_err(|_| crate::lowir::Error::Limit)?;
        let digits = symbols.literal_spelling(literal).get(..digits).ok_or(
            crate::sema::Error::Invariant("numeric literal digits outside spelling"),
        )?;
        units.extend(digits.iter().copied().map(u64::from));
        units.push(0);
        return string_object(program, symbols, crate::lowir::vocabulary::Type::I8, units);
    }
    let crate::sema::lowering::TypeView::Array { element, .. } =
        graph.lower_type(graph.lower_literal_storage(expr)?)?
    else {
        return Err(crate::sema::Error::Invariant("string address without array").into());
    };
    string(program, symbols, graph, literal, element)
}
pub(super) fn string(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    literal: ast::Literal,
    element: crate::sema::lowering::TypeId,
) -> Result<SymbolId, Error> {
    let PostTokenKind::String { ty, .. } = symbols.literal_kind(literal) else {
        return Err(Error::Missing("non-string literal object"));
    };
    let storage = types::storage(graph, element)?;
    let mut units = symbols.data.take_string_units();
    units.extend(
        symbols
            .literal_bytes(literal)
            .chunks_exact(ty.size())
            .map(|unit| {
                let mut bytes = [0u8; 8];
                bytes[..unit.len()].copy_from_slice(unit);
                u64::from_le_bytes(bytes)
            }),
    );
    string_object(program, symbols, storage, units)
}
pub(super) fn string_object(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    storage: crate::lowir::vocabulary::Type,
    units: Vec<u64>,
) -> Result<SymbolId, Error> {
    let key = (storage, units);
    if let Some(&symbol) = symbols.data.strings.get(&key) {
        symbols.data.string_units = key.1;
        return Ok(symbol);
    }
    let items = key
        .1
        .iter()
        .map(|&unit| DataItem::Constant(storage, integer(unit)))
        .collect();
    let symbol = symbols.data_symbol(program, None)?;
    define(program, symbol, items)?;
    symbols.data.strings.insert(key, symbol);
    Ok(symbol)
}
pub(super) fn image(
    output: &mut (impl Output + ?Sized),
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    image: &[StaticItem],
) -> Result<SymbolId, Error> {
    let items = items(output, cx, image)?;
    if let Some(&symbol) = cx.symbols.data.images.get(&items) {
        return Ok(symbol);
    }
    let index = cx.symbols.data.images.len();
    let symbol = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .data_symbol(output.program(), Some(index))?;
    define(output.program(), symbol, items.clone())?;
    cx.symbols.data.images.insert(items, symbol);
    Ok(symbol)
}
fn define(program: &mut Program, symbol: SymbolId, items: Vec<DataItem>) -> Result<(), Error> {
    let name = program.symbol(symbol).name;
    program.add_global(
        symbol,
        Global {
            name,
            ty: None,
            metadata: Metadata {
                binding: Some(Binding::Internal),
                storage: Some(Storage::ReadOnly),
                ..Metadata::default()
            },
            init: GlobalInit::Data(items),
        },
    )?;
    Ok(())
}

pub(super) fn reference_temporary(
    output: &mut (impl Output + ?Sized),
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    entity: crate::sema::lowering::EntityId,
    evaluated: bool,
) -> Result<SymbolId, Error> {
    if let Some(&symbol) = cx.symbols.data.reference_temporaries.get(&entity) {
        return Ok(symbol);
    }
    let extended = cx.graph.lower_extended_temporary(entity);
    if let Some(id) = extended
        && let Some(&symbol) = cx.symbols.data.constant_temporaries.get(&id)
    {
        cx.symbols.data.reference_temporaries.insert(entity, symbol);
        return Ok(symbol);
    }
    let facts = cx.graph.lower_variable(entity)?;
    let temporary =
        facts
            .reference_temporary
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "reference temporary type missing",
            )))?;
    let value = facts
        .initializer
        .ok_or(Error::Missing("dynamic global reference temporary"))?;
    // A forward reference's pointer value can require the constant-evaluation
    // fallback even when this is its own definition. Consume its typed root;
    // no initializer parsing or semantic search belongs here.
    let evaluated = evaluated
        || cx.graph.lower_evaluated_reference_temporary(entity)
        || matches!(value, StaticValue::Address(root, _)
        if cx.graph.lower_is_variable(root) && cx.graph.lower_variable(root)?.reference_temporary.is_some()
            && !cx.symbols.data.reference_defined(root));
    let ty = if evaluated {
        temporary.value
    } else {
        temporary.binding
    };
    // Resolve nested backing objects before allocating/naming their owner.
    let target = match value {
        StaticValue::Address(..)
        | StaticValue::Literal(..)
        | StaticValue::Temporary(..)
        | StaticValue::TypeInformation(..) => Some(address(output, cx, value)?),
        _ => None,
    };
    let (symbol, object) = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .reference_temporary_symbol(output.program(), entity, evaluated)?;
    let init = match value {
        StaticValue::MemberPointer(_) => GlobalInit::Data(member_items(
            output,
            &mut cx.symbols.view(&*cx.graph, &*cx.tree, cx.cursor),
            &*cx.graph,
            value,
        )?),
        StaticValue::Integer(0)
            if !evaluated
                && matches!(
                    cx.graph.lower_type(ty)?,
                    crate::sema::lowering::TypeView::Pointer(_)
                        | crate::sema::lowering::TypeView::Scalar(
                            crate::sema::lowering::Scalar::Nullptr
                        )
                ) =>
        {
            GlobalInit::Zero
        }
        StaticValue::Address(..)
        | StaticValue::Literal(..)
        | StaticValue::Temporary(..)
        | StaticValue::TypeInformation(..)
            if evaluated =>
        {
            let (target, offset) = target.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "temporary address missing",
            )))?;
            GlobalInit::Address(target, offset)
        }
        StaticValue::Address(..)
        | StaticValue::Literal(..)
        | StaticValue::Temporary(..)
        | StaticValue::TypeInformation(..) => GlobalInit::Zero,
        _ => GlobalInit::Constant(constant(value)?),
    };
    let name = output.program().symbol(symbol).name;
    output.program().add_global(
        symbol,
        Global {
            name,
            ty: if let GlobalInit::Data(items) = &init {
                super::globals::structured_type(
                    &*cx.graph,
                    ty,
                    items,
                    cx.symbols
                        .view(&*cx.graph, &*cx.tree, cx.cursor)
                        .object_mode(),
                )?
            } else {
                Some(types::storage(&*cx.graph, ty)?)
            },
            metadata: Metadata {
                binding: Some(Binding::Internal),
                storage: if cx.graph.lower_cv(ty) == 1
                    && matches!(init, GlobalInit::Constant(_) | GlobalInit::Data(_))
                {
                    Some(Storage::ReadOnly)
                } else {
                    None
                },
                object,
                ..Metadata::default()
            },
            init,
        },
    )?;
    if !evaluated {
        cx.symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .note_source_reference_storage(symbol);
    }
    if let Some(id) = extended {
        cx.symbols.data.constant_temporaries.insert(id, symbol);
    }
    cx.symbols.data.reference_temporaries.insert(entity, symbol);
    Ok(symbol)
}

impl super::function::Function<'_, '_, '_, '_> {
    pub(super) fn static_data_items(
        &mut self,
        image: &[StaticItem],
    ) -> Result<Vec<DataItem>, Error> {
        self.with_static_data(|output, cx| items(output, cx, image))
    }
    pub(super) fn static_data_address(
        &mut self,
        value: StaticValue,
    ) -> Result<(SymbolId, i64), Error> {
        self.with_static_data(|output, cx| address(output, cx, value))
    }
    pub(super) fn static_data_image(&mut self, value: &[StaticItem]) -> Result<SymbolId, Error> {
        self.with_static_data(|output, cx| image(output, cx, value))
    }
    pub(super) fn static_reference_temporary(
        &mut self,
        entity: crate::sema::lowering::EntityId,
        evaluated: bool,
    ) -> Result<SymbolId, Error> {
        self.with_static_data(|output, cx| reference_temporary(output, cx, entity, evaluated))
    }
}
