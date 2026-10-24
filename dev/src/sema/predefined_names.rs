// Translated from sema/pretty_function.cpp::PredefinedFunctionName and AppendType.
// See NOTICE.
//! Canonical source names are presentation data, never semantic lookup keys.
use super::{
    Error, Graph,
    function_names::SpecialName,
    graph::{EntityId, MemberKind, ScopeId, ScopeKind},
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::Identifiers, pp_lexer::Punctuator as P},
    support::stack::StackBudget,
    types::FundamentalType as F,
};

#[derive(Clone, Copy)]
pub(crate) enum SourceFunction {
    Function(EntityId),
    ImplicitConstructor(EntityId),
}

pub(super) fn function_name(
    graph: &Graph,
    identifiers: &Identifiers,
    function: SourceFunction,
    budget: StackBudget,
    stack_base: usize,
) -> Result<Vec<u8>, Error> {
    let printer = Printer {
        graph,
        identifiers,
        budget,
        base: stack_base,
    };
    match function {
        SourceFunction::Function(function) => {
            printer.function(graph.lower_closure_body_owner(function))
        }
        SourceFunction::ImplicitConstructor(class) => Ok(graph
            .entity(class)
            .name
            .map_or_else(Vec::new, |name| identifiers.bytes(name).to_vec())),
    }
}
pub(super) fn pretty_function_name(
    graph: &Graph,
    identifiers: &Identifiers,
    function: EntityId,
    budget: StackBudget,
    stack_base: usize,
) -> Result<Vec<u8>, Error> {
    Printer {
        graph,
        identifiers,
        budget,
        base: stack_base,
    }
    .pretty(function)
}
struct Printer<'a> {
    graph: &'a Graph,
    identifiers: &'a Identifiers,
    budget: StackBudget,
    base: usize,
}
impl Printer<'_> {
    fn check_stack(&self) -> Result<(), Error> {
        let marker = 0u8;
        if self
            .budget
            .exceeded(self.base, std::ptr::from_ref(&marker).addr())
        {
            return Err(Error::Limit("constructs nested too deeply"));
        }
        Ok(())
    }
    fn pretty(&self, id: EntityId) -> Result<Vec<u8>, Error> {
        self.check_stack()?;
        let node = self.graph.entity(id);
        let ty = self.graph.lower_entity_type(id)?;
        let TypeKind::Function { result, .. } = self.graph.types.get(ty).kind else {
            return Err(Error::Invariant("pretty name without function signature"));
        };
        let mut out = Vec::new();
        if node.function.is_some_and(|properties| properties.is_static)
            && node
                .home
                .is_some_and(|home| self.graph.scope(home).kind == ScopeKind::Class)
        {
            out.extend_from_slice(b"static ");
        }
        if self.graph.virtuals.function(id).is_virtual {
            out.extend_from_slice(b"virtual ");
        }
        let mut declarator = self.scope(node.home)?;
        declarator.extend(self.function(id)?);
        declarator.extend(self.parameters(ty)?);
        if node.member_function.is_some_and(|member| {
            matches!(
                member.kind,
                MemberKind::Constructor | MemberKind::Destructor
            )
        }) {
            out.extend(declarator);
        } else if self.graph.predefined_variables.placeholder_result(id)
            || self.graph.lower_call_closure(id).is_some()
        {
            out.extend_from_slice(b"auto ");
            out.extend(declarator);
        } else {
            out.extend(self.ty(result, declarator)?);
        }
        let mut classes = Vec::new();
        let mut scope = node.home;
        while let Some(id) = scope {
            let node = self.graph.scope(id);
            if node.kind == ScopeKind::Class
                && let Some(class) = node.owner
            {
                classes.push(class);
            }
            scope = node.parent;
        }
        let mut bindings = Vec::new();
        for class in classes.into_iter().rev() {
            if !self.graph.class_templates.definitions.explicit(class)
                && let Some(instance) = self.graph.class_templates.instance(class)
            {
                self.bindings(
                    self.graph.entity(instance.primary).template_head,
                    &instance.arguments,
                    &mut bindings,
                )?;
            }
        }
        if !self.graph.function_templates.specializations.contains(id)
            && let Some(primary) = self.graph.function_templates.primary(id)
            && let Some(arguments) = self.graph.function_templates.arguments.get(&id)
        {
            self.bindings(
                self.graph
                    .entity(self.graph.binding(primary).entity)
                    .template_head,
                arguments,
                &mut bindings,
            )?;
        }
        if !bindings.is_empty() {
            out.extend_from_slice(b" [");
            out.extend(bindings);
            out.push(b']');
        }
        Ok(out)
    }
    fn bindings(
        &self,
        head: Option<super::templates::HeadId>,
        arguments: &[TypeId],
        out: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let Some(head) = head else { return Ok(()) };
        for (&parameter, &argument) in self.graph.templates.parameters(head).iter().zip(arguments) {
            let Some(name) = self.graph.entity(parameter).name else {
                continue;
            };
            if !out.is_empty() {
                out.extend_from_slice(b", ");
            }
            out.extend_from_slice(self.identifiers.bytes(name));
            out.extend_from_slice(b" = ");
            out.extend(self.argument(argument)?);
        }
        Ok(())
    }
    fn function(&self, id: EntityId) -> Result<Vec<u8>, Error> {
        self.check_stack()?;
        let node = self.graph.entity(id);
        if let Some(member) = node.member_function {
            if matches!(
                member.kind,
                MemberKind::Constructor | MemberKind::Destructor
            ) {
                let mut out = Vec::new();
                if member.kind == MemberKind::Destructor {
                    out.push(b'~');
                }
                if let Some(name) = self.graph.entity(member.class).name {
                    out.extend_from_slice(self.identifiers.bytes(name));
                }
                return Ok(out);
            }
            if member.kind == MemberKind::Conversion {
                let TypeKind::Function { result, .. } =
                    self.graph.types.get(self.graph.lower_entity_type(id)?).kind
                else {
                    return Err(Error::Invariant("conversion without signature"));
                };
                let mut out = b"operator ".to_vec();
                out.extend(self.ty(result, Vec::new())?);
                return Ok(out);
            }
        }
        match node.function.and_then(|function| function.special) {
            Some(SpecialName::Operator { token, array }) => {
                let mut out = b"operator".to_vec();
                if token.as_keyword().is_some() {
                    out.push(b' ');
                }
                out.extend_from_slice(token.spelling().as_bytes());
                match token.as_punctuator() {
                    Some(P::LeftParen) => out.push(b')'),
                    Some(P::LeftBracket) => out.push(b']'),
                    _ => {}
                }
                if array {
                    out.extend_from_slice(b"[]");
                }
                Ok(out)
            }
            Some(SpecialName::Literal(name)) => {
                let mut out = b"operator\"\"".to_vec();
                out.extend_from_slice(self.identifiers.bytes(name));
                Ok(out)
            }
            _ => Ok(node
                .name
                .map_or_else(Vec::new, |name| self.identifiers.bytes(name).to_vec())),
        }
    }
    fn scope(&self, mut id: Option<ScopeId>) -> Result<Vec<u8>, Error> {
        self.check_stack()?;
        while let Some(scope) = id {
            let node = self.graph.scope(scope);
            if node.kind == ScopeKind::Class {
                let mut out = self.class(
                    node.owner
                        .ok_or(Error::Invariant("class scope without owner"))?,
                )?;
                out.extend_from_slice(b"::");
                return Ok(out);
            }
            if node.kind == ScopeKind::Namespace {
                if scope == self.graph.global {
                    return Ok(Vec::new());
                }
                let mut out = self.scope(node.parent)?;
                if let Some(name) = node.name {
                    if !node.is_inline {
                        out.extend_from_slice(self.identifiers.bytes(name));
                        out.extend_from_slice(b"::");
                    }
                } else {
                    out.extend_from_slice(b"(anonymous namespace)::");
                }
                return Ok(out);
            }
            if node.kind == ScopeKind::Function
                && let Some(function) = node.owner
            {
                let mut out = self.scope(self.graph.entity(function).home)?;
                out.extend(self.function(function)?);
                out.extend(self.parameters(self.graph.lower_entity_type(function)?)?);
                out.extend_from_slice(b"::");
                return Ok(out);
            }
            id = node.parent;
        }
        Ok(Vec::new())
    }
    fn class(&self, id: EntityId) -> Result<Vec<u8>, Error> {
        self.check_stack()?;
        let node = self.graph.entity(id);
        let mut out = self.scope(node.home)?;
        if let Some(name) = node.name {
            out.extend_from_slice(self.identifiers.bytes(name));
        } else {
            out.extend_from_slice(match node.key {
                crate::lex::vocabulary::Keyword::Union => b"(anonymous union)",
                crate::lex::vocabulary::Keyword::Class => b"(anonymous class)",
                _ => b"(anonymous struct)",
            });
            return Ok(out);
        }
        if let Some(instance) = self.graph.class_templates.instance(id) {
            out.push(b'<');
            let mut first = true;
            for &argument in &instance.arguments {
                let arguments =
                    if let TypeKind::Pack(elements) = &self.graph.types.get(argument).kind {
                        elements.as_slice()
                    } else {
                        std::slice::from_ref(&argument)
                    };
                for &argument in arguments {
                    if !first {
                        out.extend_from_slice(b", ");
                    }
                    first = false;
                    out.extend(self.argument(argument)?);
                }
            }
            out.push(b'>');
        }
        Ok(out)
    }
    fn parameters(&self, ty: TypeId) -> Result<Vec<u8>, Error> {
        self.check_stack()?;
        let TypeKind::Function {
            parameters,
            variadic,
            cv,
            reference,
            ..
        } = &self.graph.types.get(ty).kind
        else {
            return Err(Error::Invariant("source parameters without function"));
        };
        let mut out = vec![b'('];
        for (i, &parameter) in parameters.iter().enumerate() {
            if i != 0 {
                out.extend_from_slice(b", ");
            }
            let text = match self.graph.types.get(parameter).kind {
                TypeKind::Array { element, .. } => self.ty(
                    element,
                    if matches!(self.graph.types.get(element).kind, TypeKind::Array { .. }) {
                        b"(*)".to_vec()
                    } else {
                        vec![b'*']
                    },
                )?,
                TypeKind::Function { .. } => self.ty(parameter, b"(*)".to_vec())?,
                _ => self.ty(parameter, Vec::new())?,
            };
            out.extend(text);
        }
        if *variadic {
            if !parameters.is_empty() {
                out.extend_from_slice(b", ");
            }
            out.extend_from_slice(b"...");
        }
        out.push(b')');
        if cv.0 & 1 != 0 {
            out.extend_from_slice(b" const");
        }
        if cv.0 & 2 != 0 {
            out.extend_from_slice(b" volatile");
        }
        if let Some(reference) = reference {
            out.extend_from_slice(if *reference == RefKind::Lvalue {
                b" &"
            } else {
                b" &&"
            });
        }
        Ok(out)
    }
    fn cv(cv: Cv) -> Vec<u8> {
        let mut out = Vec::new();
        if cv.0 & 1 != 0 {
            out.extend_from_slice(b"const ");
        }
        if cv.0 & 2 != 0 {
            out.extend_from_slice(b"volatile ");
        }
        if cv.0 & Cv::ATOMIC.0 != 0 {
            out.extend_from_slice(b"_Atomic ");
        }
        out
    }
    fn scalar(scalar: Scalar) -> &'static str {
        match scalar {
            Scalar::Short => "short",
            Scalar::UnsignedShort => "unsigned short",
            Scalar::Nullptr => "std::nullptr_t",
            Scalar::Basic(F::LongInt) => "long",
            Scalar::Basic(F::UnsignedLongInt) => "unsigned long",
            Scalar::Basic(F::LongLongInt) => "long long",
            Scalar::Basic(F::UnsignedLongLongInt) => "unsigned long long",
            _ => scalar.name(),
        }
    }
    fn ty(&self, mut ty: TypeId, mut inner: Vec<u8>) -> Result<Vec<u8>, Error> {
        self.check_stack()?;
        loop {
            let node = self.graph.types.get(ty);
            let (element, mut prefix) = match node.kind {
                TypeKind::Pointer(element) => (element, vec![b'*']),
                TypeKind::BlockPointer(element) => (element, vec![b'^']),
                TypeKind::MemberPointer { owner, element } => {
                    let mut prefix = self.ty(owner, Vec::new())?;
                    prefix.extend_from_slice(b"::*");
                    (element, prefix)
                }
                TypeKind::Reference { element, kind } => (
                    element,
                    if kind == RefKind::Lvalue {
                        b"&".to_vec()
                    } else {
                        b"&&".to_vec()
                    },
                ),
                TypeKind::Array { element, bound, .. } => {
                    inner.extend_from_slice(
                        if bound == 0 {
                            "[]".to_owned()
                        } else {
                            format!("[{bound}]")
                        }
                        .as_bytes(),
                    );
                    ty = element;
                    continue;
                }
                TypeKind::Function { result, .. } => {
                    inner.extend(self.parameters(ty)?);
                    ty = result;
                    continue;
                }
                _ => break,
            };
            if node.cv.0 != 0 {
                let mut cv = Self::cv(node.cv);
                let _space = cv.pop();
                prefix.extend(cv);
                if !inner.is_empty() {
                    prefix.push(b' ');
                }
            }
            prefix.extend(inner);
            inner = prefix;
            if matches!(
                self.graph.types.get(element).kind,
                TypeKind::Array { .. } | TypeKind::Function { .. }
            ) {
                inner.insert(0, b'(');
                inner.push(b')');
            }
            ty = element;
        }
        let node = self.graph.types.get(ty);
        let mut out = Self::cv(node.cv);
        match &node.kind {
            TypeKind::Scalar(scalar) => out.extend_from_slice(Self::scalar(*scalar).as_bytes()),
            TypeKind::Vector { element, bound, .. } => {
                out.extend_from_slice(
                    format!("__attribute__((__vector_size__({bound} * sizeof(").as_bytes(),
                );
                let lane = self.ty(*element, Vec::new())?;
                out.extend_from_slice(&lane);
                out.extend_from_slice(b")))) ");
                out.extend(lane);
            }
            TypeKind::Complex(part) => {
                out.extend_from_slice(b"_Complex ");
                out.extend(self.ty(*part, Vec::new())?);
            }
            TypeKind::Class(class) => out.extend(self.class(*class)?),
            TypeKind::Enum(entity) => {
                let node = self.graph.entity(*entity);
                out.extend(self.scope(node.home)?);
                out.extend_from_slice(node.name.map_or(b"(anonymous enum)".as_slice(), |name| {
                    self.identifiers.bytes(name)
                }));
            }
            TypeKind::Expansion(pattern) => {
                out.extend(self.ty(*pattern, Vec::new())?);
                out.extend_from_slice(b"...");
            }
            TypeKind::BoundPack { arguments, .. } => out.extend(self.argument(*arguments)?),
            TypeKind::Value { .. }
            | TypeKind::Address { .. }
            | TypeKind::Pack(_)
            | TypeKind::Template(_) => out.extend(self.argument(ty)?),
            TypeKind::TemplateParameter { entity, .. } => out.extend_from_slice(
                self.graph
                    .entity(*entity)
                    .name
                    .map_or(b"auto".as_slice(), |name| self.identifiers.bytes(name)),
            ),
            TypeKind::Auto => out.extend_from_slice(b"auto"),
            _ => return Err(Error::Invariant("source type suffix remained")),
        }
        if !inner.is_empty() {
            if inner[0] != b'[' {
                out.push(b' ');
            }
            out.extend(inner);
        }
        Ok(out)
    }
    fn argument(&self, ty: TypeId) -> Result<Vec<u8>, Error> {
        self.check_stack()?;
        match &self.graph.types.get(ty).kind {
            TypeKind::Template(id) | TypeKind::Address { entity: id, .. } => {
                let node = self.graph.entity(*id);
                let mut out = Vec::new();
                if let TypeKind::Address { ty, .. } = self.graph.types.get(ty).kind
                    && !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                {
                    out.push(b'&');
                }
                out.extend(self.scope(node.home)?);
                if let Some(name) = node.name {
                    out.extend_from_slice(self.identifiers.bytes(name));
                }
                Ok(out)
            }
            TypeKind::Pack(elements) => {
                let mut out = vec![b'<'];
                for (i, &element) in elements.iter().enumerate() {
                    if i != 0 {
                        out.extend_from_slice(b", ");
                    }
                    out.extend(self.argument(element)?);
                }
                out.push(b'>');
                Ok(out)
            }
            TypeKind::Value { ty, bits } => self.value(*ty, *bits),
            _ => self.ty(ty, Vec::new()),
        }
    }
    fn value(&self, mut ty: TypeId, bits: u64) -> Result<Vec<u8>, Error> {
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Pointer(_)
                | TypeKind::MemberPointer { .. }
                | TypeKind::Reference { .. }
                | TypeKind::Scalar(Scalar::Nullptr)
        ) {
            return Ok(b"nullptr".to_vec());
        }
        let mut out = Vec::new();
        if let TypeKind::Enum(id) = self.graph.types.get(ty).kind {
            let node = self.graph.entity(id);
            let info = node
                .enumeration
                .ok_or(Error::Invariant("enum without facts"))?;
            let scope = if info.scoped { node.scope } else { node.home };
            let enumerator = scope.and_then(|scope| self.graph.scope(scope).bindings.iter().copied().find(|&binding| {
                let entity = self.graph.entity(self.graph.binding(binding).entity);
                self.graph.binding(binding).kind == super::graph::BindingKind::Enumerator && matches!(entity.constant, super::constants::ConstantState::Value(value) if value.ty == ty && value.bits == bits)
            }));
            if let Some(binding) = enumerator {
                let entity = self.graph.entity(self.graph.binding(binding).entity);
                if info.scoped {
                    out.extend(self.ty(ty, Vec::new())?);
                    out.extend_from_slice(b"::");
                } else {
                    out.extend(self.scope(entity.home)?);
                }
                if let Some(name) = entity.name {
                    out.extend_from_slice(self.identifiers.bytes(name));
                }
                return Ok(out);
            }
            out.push(b'(');
            out.extend(self.ty(ty, Vec::new())?);
            out.push(b')');
            ty = info.underlying.unwrap_or(ty);
        }
        let negative = self.graph.integral_kind(ty).is_some_and(|kind| kind.signed)
            && super::integral::signed(bits) < 0;
        let digits = if negative {
            format!("-{}", bits.wrapping_neg())
        } else {
            bits.to_string()
        };
        let TypeKind::Scalar(scalar) = self.graph.types.get(ty).kind else {
            out.extend_from_slice(digits.as_bytes());
            return Ok(out);
        };
        let text = match scalar {
            Scalar::Bool => {
                if bits != 0 {
                    "true".to_owned()
                } else {
                    "false".to_owned()
                }
            }
            Scalar::Basic(F::Char | F::Char16T | F::Char32T | F::WcharT) => {
                let prefix = match scalar {
                    Scalar::Basic(F::Char16T) => "u",
                    Scalar::Basic(F::Char32T) => "U",
                    Scalar::Basic(F::WcharT) => "L",
                    _ => "",
                };
                let c = u8::try_from(bits)
                    .ok()
                    .filter(|&c| (32..127).contains(&c) && !matches!(c, b'\'' | b'\\'));
                format!(
                    "{prefix}'{}'",
                    c.map_or_else(|| format!("\\x{bits:x}"), |c| char::from(c).to_string())
                )
            }
            Scalar::Basic(F::Int) => digits,
            Scalar::Basic(F::UnsignedInt) => digits + "U",
            Scalar::Basic(F::LongInt) => digits + "L",
            Scalar::Basic(F::UnsignedLongInt) => digits + "UL",
            Scalar::Basic(F::LongLongInt) => digits + "LL",
            Scalar::Basic(F::UnsignedLongLongInt) => digits + "ULL",
            _ => format!("({}){digits}", Self::scalar(scalar)),
        };
        out.extend_from_slice(text.as_bytes());
        Ok(out)
    }
}
