// Translated from lower/allocation_lowering.cpp::array_allocation/new_list_elements.
// See NOTICE.
//! Cookie counts and initialized prefixes belong to explicit storage.
use super::{Error, array_allocation::Shape, function::Function, types};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Type},
    },
    sema::lowering::{AllocationView, ArrayAllocation, EntityId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

#[derive(Clone, Copy)]
pub(super) struct Storage {
    pub raw: Value,
    pub size: Value,
    pub known_size: Value,
}
#[derive(Clone, Copy)]
struct Prefix<'a> {
    array: ArrayAllocation,
    elements: Value,
    storage: &'a Storage,
    values: &'a [Value],
    rest_throws: bool,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn array_cookie_bytes(
        &self,
        element: TypeId,
        sized: bool,
        reserved: bool,
    ) -> Result<u64, Error> {
        let TypeView::Class(class) = self.graph.lower_type(element)? else {
            return Ok(0);
        };
        if reserved {
            return Ok(0);
        }
        if sized || self.graph.lower_nontrivial_destructor(class)? {
            let (_, alignment) = types::layout(&*self.graph, element)?;
            Ok(8_u64.max(1_u64 << alignment))
        } else {
            Ok(0)
        }
    }
    pub(super) fn initialize_allocated_array(
        &mut self,
        id: ast::ExprId,
        use_: &AllocationView,
        array: ArrayAllocation,
        shape: &Shape,
        storage: Storage,
        values: &[Value],
    ) -> Result<Value, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let ExprKind::New { initializer, .. } = self.tree.expressions.get(id).kind else {
            return Err(crate::sema::Error::Invariant("array initialization syntax").into());
        };
        let class = matches!(self.graph.lower_type(shape.element)?, TypeView::Class(_));
        let elements = if shape.cookie_bytes == 0 {
            storage.raw
        } else {
            let cookie = self.builder.integer(u128::from(shape.cookie_bytes))?;
            self.emit(I::Index(Type::I8, None, storage.raw, cookie), source)?
                .value
        };
        let count = if shape.cookie_bytes != 0 {
            let count = if let Some(count) = array.constant {
                self.array_count_constant(source, count.wrapping_mul(shape.per_element))?
            } else {
                let bytes =
                    self.array_size_subtract(source, storage.known_size, shape.cookie_bytes)?;
                self.array_size_divide(source, bytes, shape.element_size)?
            };
            let header = if shape.cookie_bytes == 8 {
                storage.raw
            } else {
                let offset = self.builder.integer(u128::from(shape.cookie_bytes - 8))?;
                self.emit(I::Index(Type::I8, None, storage.raw, offset), source)?
                    .value
            };
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                count,
                header,
            ))?;
            Some(count)
        } else if class && array.constant.is_none() {
            Some(self.array_size_divide(source, storage.known_size, shape.element_size)?)
        } else {
            None
        };
        let constructor = use_.constructor.map(|(id, _)| id);
        let constructs = if let Some(id) = constructor {
            !self.graph.lower_constructor_trivial(id) && !self.array_constructor_inert(id)?
        } else {
            false
        };
        let trailing = array.constant.is_none_or(|count| count > shape.listed);
        let rest_throws = if trailing
            && constructs
            && let Some(id) = constructor
        {
            !self.graph.lower_constructor_function(id)?.nothrow
        } else {
            false
        };
        let freed = self.initialize_array_prefix(
            id,
            use_,
            shape,
            Prefix {
                array,
                elements,
                storage: &storage,
                values,
                rest_throws,
            },
        )?;
        let listed_bytes = shape.listed.wrapping_mul(shape.stride);
        let rest = if listed_bytes == 0 {
            elements
        } else {
            let offset = self.builder.integer(u128::from(listed_bytes))?;
            self.emit(I::Index(Type::I8, None, elements, offset), source)?
                .value
        };
        let value_init = initializer.is_some();
        if trailing
            && value_init
            && constructor.is_none_or(|id| self.graph.lower_constructor(id).zero_before_value)
        {
            let bytes = if let Some(count) = array.constant {
                self.builder.integer(u128::from(
                    count
                        .wrapping_mul(shape.stride)
                        .wrapping_add(shape.cookie_bytes),
                ))?
            } else {
                storage.known_size
            };
            let bytes = self.array_size_subtract(source, bytes, shape.cookie_bytes)?;
            let bytes = self.array_size_subtract(source, bytes, listed_bytes)?;
            self.zero_allocated_bytes(source, rest, bytes)?;
        }
        if trailing
            && value_init
            && self.graph.lower_holds_data_member_pointer(shape.element)?
            && constructor.is_none_or(|id| self.graph.lower_constructor(id).zero_before_value)
        {
            let remaining = if let Some(count) = array.constant {
                self.array_count_constant(
                    source,
                    count
                        .wrapping_sub(shape.listed)
                        .wrapping_mul(shape.per_element),
                )?
            } else {
                let all = if let Some(count) = count {
                    count
                } else {
                    self.array_size_divide(source, storage.known_size, shape.element_size)?
                };
                self.array_size_subtract(source, all, shape.listed.wrapping_mul(shape.per_element))?
            };
            self.null_member_elements(shape.element, rest, remaining)?;
        }
        if constructs && (trailing || array.constant == Some(0)) {
            let count = if let Some(count) = array.constant {
                self.array_count_constant(
                    source,
                    count
                        .wrapping_sub(shape.listed)
                        .wrapping_mul(shape.per_element),
                )?
            } else {
                self.array_size_subtract(
                    source,
                    count.ok_or(crate::sema::Error::Invariant("class array count"))?,
                    shape.listed.wrapping_mul(shape.per_element),
                )?
            };
            self.construct_allocated_elements(
                super::array_construction::Construction {
                    location: self.tree.expressions.get(id).location,
                    element: shape.element,
                    elements: rest,
                    count,
                    raw: storage.raw,
                    size: storage.size,
                    constructor: constructor
                        .ok_or(crate::sema::Error::Invariant("array construction plan"))?,
                    deallocator: if freed.is_some() {
                        None
                    } else {
                        use_.deallocator
                    },
                },
                values,
            )?;
        }
        if let Some(first) = freed {
            self.close_lifetime_region()?;
            self.release_aggregate_members(first);
        }
        Ok(elements)
    }
    pub(super) fn array_count_constant(
        &mut self,
        source: TypeId,
        count: u64,
    ) -> Result<Value, Error> {
        let value = self.builder.integer(u128::from(count))?;
        Ok(self.emit(I::Const(Type::I64, value), source)?.value)
    }

    fn array_size_divide(
        &mut self,
        source: TypeId,
        bytes: Value,
        size: u64,
    ) -> Result<Value, Error> {
        let size = self.builder.integer(u128::from(size))?;
        Ok(self
            .emit(
                I::Binary(Binary::UDiv, Facts::default(), Type::I64, bytes, size),
                source,
            )?
            .value)
    }
    fn array_size_subtract(
        &mut self,
        source: TypeId,
        bytes: Value,
        subtract: u64,
    ) -> Result<Value, Error> {
        if subtract == 0 {
            return Ok(bytes);
        }
        let subtract = self.builder.integer(u128::from(subtract))?;
        Ok(self
            .emit(
                I::Binary(Binary::Sub, Facts::default(), Type::I64, bytes, subtract),
                source,
            )?
            .value)
    }
    fn initialize_array_prefix(
        &mut self,
        id: ast::ExprId,
        use_: &AllocationView,
        shape: &Shape,
        prefix: Prefix<'_>,
    ) -> Result<Option<usize>, Error> {
        let Prefix {
            array,
            elements,
            storage,
            values,
            rest_throws,
        } = prefix;
        if shape.listed == 0 {
            return Ok(None);
        }
        let ExprKind::New {
            initializer:
                Some(ast::Initializer {
                    kind: ast::InitializerKind::Braced(list),
                    ..
                }),
            ..
        } = self.tree.expressions.get(id).kind
        else {
            return Err(crate::sema::Error::Invariant("array prefix without list").into());
        };
        let ExprKind::Braced {
            elements: list_elements,
            ..
        } = self.tree.expressions.get(list).kind
        else {
            return Err(crate::sema::Error::Invariant("array prefix list syntax").into());
        };
        let prefix = array
            .prefix
            .ok_or(crate::sema::Error::Invariant("array prefix type"))?;
        let mut throws = self.expression_throws(list)? || rest_throws;
        if let TypeView::Class(class) = self.graph.lower_type(shape.element)? {
            throws |= self.omitted_aggregate_throws(class, 0)?;
        }
        let freed = if throws && let Some(function) = use_.deallocator {
            let first = self.lifetime.cleanups.len();
            let args =
                self.array_deallocation_arguments(function, storage.raw, storage.size, values)?;
            self.register_storage_cleanup(prefix, storage.raw, function, Some(args))?;
            Some(first)
        } else {
            None
        };
        let saved_extension = self.reference_lifetimes.aggregate.take();
        let saved_count = self.aggregation.count.take();
        let saved_floor = self.aggregation.floor;
        let saved_regions = std::mem::take(&mut self.aggregation.regions);
        self.aggregation.floor = freed.map(|_| self.lifetime.cleanups.len());
        let initialized = self.allocated_array_prefix(
            prefix,
            elements,
            &self.owned_expression_ids(list_elements),
        );
        self.aggregation.floor = saved_floor;
        self.aggregation.regions = saved_regions;
        self.aggregation.count = saved_count;
        self.reference_lifetimes.aggregate = saved_extension;
        initialized?;
        Ok(freed)
    }
    pub(super) fn array_deallocation_arguments(
        &mut self,
        function: EntityId,
        raw: Value,
        size: Value,
        placement: &[Value],
    ) -> Result<crate::lowir::model::Arguments, Error> {
        let ty = self.graph.lower_function(function)?.ty;
        let TypeView::Function { parameters, .. } = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("array deallocation signature").into());
        };
        let parameters = parameters.to_vec();
        let parameters = parameters.as_slice();
        let mut values = vec![raw];
        for index in 1..parameters.len() {
            values.push(placement.get(index).copied().unwrap_or(size));
        }
        Ok(self.builder.arguments(&values)?)
    }
}
