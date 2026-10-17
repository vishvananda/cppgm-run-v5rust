// Translated from symbol_lowering.cpp::append_class_label/append_type_label.
// See NOTICE.
//! Bounded presentation labels; canonical identities remain semantic/ABI facts.
use super::{Error, Symbols};
use crate::sema::lowering::{EntityId, Scalar, TypeId, TypeView};
use std::{collections::HashMap, fmt::Write};

const LABEL_LIMIT: usize = 96;
#[derive(Default)]
pub(super) struct Labels {
    classes: HashMap<EntityId, String>,
    identities: HashMap<EntityId, u32>,
}
impl Symbols<'_, '_> {
    pub(super) fn specialized_class_label(&mut self, class: EntityId) -> Result<String, Error> {
        if let Some(label) = self.class_labels.classes.get(&class) {
            return Ok(label.clone());
        }
        let (_, home, _) = self
            .graph
            .lower_nominal_name(self.graph.lower_entity_type(class)?)?;
        if let Some(outer) = self.graph.lower_scope_class(home) {
            self.specialized_class_label(outer)?;
        }
        let base = self.class_label(self.graph.lower_class_spelling(class)?)?;
        let mut label = base.clone();
        if self.graph.lower_class_specialization(class) {
            label.push('<');
            for (index, ty) in self
                .graph
                .lower_class_arguments(class)
                .unwrap_or_default()
                .into_iter()
                .enumerate()
            {
                if index != 0 {
                    label.push_str(", ");
                }
                self.append_type_label(ty, &mut label, LABEL_LIMIT)?;
            }
            label.push('>');
            if label.len() > LABEL_LIMIT {
                label = base;
                label.push('<');
                self.append_label_identity(class, &mut label)?;
                label.push('>');
            }
        }
        let label = label
            .bytes()
            .filter(|&byte| byte != b' ')
            .map(|byte| {
                char::from(if byte.is_ascii_alphanumeric() || byte == b'_' {
                    byte
                } else {
                    b'_'
                })
            })
            .collect::<String>();
        self.class_labels.classes.insert(class, label.clone());
        Ok(label)
    }
    fn append_label_identity(&mut self, class: EntityId, out: &mut String) -> Result<(), Error> {
        let next = u32::try_from(self.class_labels.identities.len())
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or(crate::sema::Error::Limit("class label identity limit"))?;
        let ordinal = self.class_labels.identities.entry(class).or_insert(next);
        write!(out, "__cppgm_class_template_identity_{ordinal}")
            .map_err(|_| Error::Missing("class label formatting"))
    }
    fn append_type_label(
        &mut self,
        ty: TypeId,
        out: &mut String,
        limit: usize,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if out.len() > limit {
            return Ok(());
        }
        if let Some(elements) = self.graph.lower_argument_pack(ty) {
            for (index, &element) in elements.iter().enumerate() {
                if out.len() > limit {
                    break;
                }
                if index != 0 {
                    out.push_str(", ");
                }
                self.append_type_label(element, out, limit)?;
            }
            return Ok(());
        }
        if let Some(pattern) = self.graph.lower_pack_expansion(ty) {
            self.append_type_label(pattern, out, limit)?;
            out.push_str("...");
            return Ok(());
        }
        if let Some((_, name)) = self.graph.lower_template_name(ty) {
            out.push_str(self.identifier(name)?);
            return Ok(());
        }
        let view = self.graph.lower_type(ty)?;
        let cv = self.graph.lower_cv(ty);
        if !matches!(view, TypeView::Pointer(_) | TypeView::Array { .. }) {
            if cv & 1 != 0 {
                out.push_str("const ");
            }
            if cv & 2 != 0 {
                out.push_str("volatile ");
            }
        }
        match view {
            TypeView::Value { ty, bits } => {
                if matches!(
                    self.graph.lower_type(ty)?,
                    TypeView::Pointer(_) | TypeView::Scalar(Scalar::Nullptr)
                ) || self.graph.lower_member_pointer_type(ty).is_some()
                {
                    out.push_str("nullptr");
                    return Ok(());
                }
                if matches!(self.graph.lower_type(ty)?, TypeView::Enum(_)) {
                    out.push('(');
                    self.append_type_label(ty, out, limit)?;
                    out.push(')');
                }
                if matches!(self.graph.lower_type(ty)?, TypeView::Scalar(Scalar::Bool)) {
                    out.push_str(if bits == 0 { "false" } else { "true" });
                } else if self.graph.lower_signed_integral(ty) {
                    write!(out, "{}", bits.cast_signed())
                        .map_err(|_| Error::Missing("class label formatting"))?;
                } else {
                    write!(out, "{bits}").map_err(|_| Error::Missing("class label formatting"))?;
                }
            }
            TypeView::Scalar(scalar) => out.push_str(match scalar {
                Scalar::Basic(kind) => kind.name(),
                Scalar::Void => "void",
                Scalar::Bool => "bool",
                Scalar::SignedChar => "signed char",
                Scalar::UnsignedChar => "unsigned char",
                Scalar::Short => "short int",
                Scalar::UnsignedShort => "unsigned short int",
                Scalar::Nullptr => "nullptr_t",
            }),
            TypeView::Class(class) if self.graph.lower_class_specialization(class) => {
                self.append_label_identity(class, out)?;
            }
            TypeView::Class(class) => {
                out.push_str(&self.class_label(self.graph.lower_class_spelling(class)?)?);
            }
            TypeView::Enum(_) => {
                let (_, _, name) = self.graph.lower_nominal_name(ty)?;
                if let Some(name) = name {
                    out.push_str(self.identifier(name)?);
                }
            }
            TypeView::Pointer(element) => {
                self.append_type_label(element, out, limit)?;
                out.push('*');
                if cv & 1 != 0 {
                    out.push_str(" const");
                }
                if cv & 2 != 0 {
                    out.push_str(" volatile");
                }
            }
            TypeView::Reference { element, rvalue } => {
                self.append_type_label(element, out, limit)?;
                out.push_str(if rvalue { "&&" } else { "&" });
            }
            TypeView::Array { element, bound } => {
                self.append_type_label(element, out, limit)?;
                write!(out, "[{bound}]").map_err(|_| Error::Missing("class label formatting"))?;
            }
            TypeView::Function {
                result,
                parameters,
                variadic,
            } => {
                self.append_type_label(result, out, limit)?;
                out.push('(');
                for (index, &parameter) in parameters.iter().enumerate() {
                    if out.len() > limit {
                        break;
                    }
                    if index != 0 {
                        out.push_str(", ");
                    }
                    self.append_type_label(parameter, out, limit)?;
                }
                if variadic {
                    out.push_str(if parameters.is_empty() {
                        "..."
                    } else {
                        ", ..."
                    });
                }
                out.push(')');
            }
            TypeView::Unsupported => {
                if let Some((owner, element)) = self.graph.lower_member_pointer_type(ty) {
                    self.append_type_label(element, out, limit)?;
                    out.push(' ');
                    self.append_type_label(owner, out, limit)?;
                    out.push_str("::*");
                } else if let Some(name) = self.graph.lower_type_parameter_name(ty) {
                    out.push_str(self.identifier(name)?);
                } else {
                    return Err(Error::Missing("class argument presentation type"));
                }
            }
        }
        Ok(())
    }
}
