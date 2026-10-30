// Translated from lower/intrinsic_lowering.cpp and expression_lowering.cpp. See NOTICE.
//! Expand semantic stack/variadic operations into the existing typed runtime instructions.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{InstructionKind as I, RuntimeInstruction as Runtime, Value},
        vocabulary::{EightbyteClass, ObjectClasses, Type},
    },
    sema::lowering::{StackIntrinsic, TypeView},
    syntax::{ast, storage::List},
};
impl Function<'_, '_, '_, '_> {
    fn va_list_tag(&mut self, operand: ast::ExprId) -> Result<Value, Error> {
        let ty = self.graph.lower_expression(operand)?.ty;
        if matches!(self.graph.lower_type(ty)?, TypeView::Pointer(_)) {
            Ok(self.operand(operand, Context::Operand)?.value)
        } else {
            self.address(operand)
        }
    }
    pub(super) fn stack_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        operation: StackIntrinsic,
    ) -> Result<RValue, Error> {
        let arguments = self.owned_expression_ids(arguments);
        let arguments = arguments.as_slice();
        let source = self.graph.lower_expression(id)?.ty;
        match operation {
            StackIntrinsic::Alloca => {
                let size = self.operand(arguments[0], Context::Argument)?;
                self.emit(I::Runtime(Runtime::StackAlloc(size.value)), source)
            }
            StackIntrinsic::VaStart => {
                let list = self.va_list_tag(arguments[0])?;
                self.effect(I::Runtime(Runtime::VaStart(list)))?;
                self.zero(source)
            }
            StackIntrinsic::VaEnd => {
                self.address(arguments[0])?;
                self.zero(source)
            }
            StackIntrinsic::VaCopy => {
                let destination = self.va_list_tag(arguments[0])?;
                let source_value = self.va_list_tag(arguments[1])?;
                let mut ty = self.graph.lower_expression(arguments[0])?.ty;
                while let TypeView::Pointer(element) | TypeView::Array { element, .. } =
                    self.graph.lower_type(ty)?
                {
                    ty = element;
                }
                let (bytes, align) = types::layout(&*self.graph, ty)?;
                self.effect(I::CopyObject(bytes, align, source_value, destination))?;
                self.zero(source)
            }
        }
    }
    pub(super) fn va_arg(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(id)?.ty;
        let ty = types::storage(&*self.graph, source)?;
        let list = self.va_list_tag(operand)?;
        if let TypeView::Class(class) = self.graph.lower_type(source)?
            && !(self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .object_mode()
                && self.graph.lower_direct_class_parameter(class)?)
        {
            let address = self
                .emit(I::Runtime(Runtime::VaArg(Type::Ptr, list, None)), source)?
                .value;
            return self.emit(
                I::Load(crate::lowir::model::Facts::default(), ty, address),
                source,
            );
        }
        let wide = ty == Type::I128
            && self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .object_mode();
        let read = if wide {
            Type::object(
                16,
                if self.graph.lower_member_pointer_type(source).is_some() {
                    3
                } else {
                    4
                },
            )
        } else {
            ty
        };
        let classes = if self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .object_mode()
        {
            self.symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .object_argument_classes(source)?
                .map(|classes| match classes {
                    ObjectClasses::Direct(first, None) => {
                        ObjectClasses::Direct(first, Some(EightbyteClass::None))
                    }
                    other => other,
                })
        } else {
            None
        };
        let value = self.emit(I::Runtime(Runtime::VaArg(read, list, classes)), source)?;
        if !wide {
            return Ok(value);
        }
        let slot = self.generated_slot("arg", read)?;
        self.effect(I::Store(
            crate::lowir::model::Facts::default(),
            read,
            value.value,
            slot,
        ))?;
        let address = self.emit(I::Addr(slot), source)?.value;
        self.read_place(address, source)
    }
}
