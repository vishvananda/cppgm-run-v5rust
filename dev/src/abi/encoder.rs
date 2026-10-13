//! Itanium grammar and per-name substitution state, translated from `abi_encoder.cpp`. See NOTICE.
use super::{
    Error,
    graph::{Graph, Modifier, Node, NodeId, Qualifiers},
    statistics::Statistics,
};
use crate::support::stack::StackBudget;

#[derive(Clone, Copy, Debug, Default)]
pub struct CallOffset {
    pub offset: i64,
    pub vcall: Option<i64>,
}
#[derive(Clone, Copy, Debug)]
pub enum Target {
    Type(NodeId),
    Entity(NodeId),
    CName(NodeId),
    Typeinfo(NodeId),
    TypeinfoName(NodeId),
    Vtable(NodeId),
    Vtt(NodeId),
    ConstructionVtable {
        derived: NodeId,
        offset: i64,
        base: NodeId,
    },
    TlsWrapper(NodeId),
    Thunk {
        function: NodeId,
        this: CallOffset,
        result: Option<CallOffset>,
    },
}
pub struct Encoder<'a> {
    graph: &'a Graph,
    slots: Vec<Option<u32>>,
    entries: u32,
    output: &'a mut Vec<u8>,
    budget: StackBudget,
    stack_base: usize,
    stats: Option<&'a mut Statistics>,
}
impl<'a> Encoder<'a> {
    /// Append one name using the graph's canonical identities.
    ///
    /// # Errors
    /// Returns an encoding diagnostic or a checked storage/stack limit.
    pub fn encode(
        graph: &'a Graph,
        target: Target,
        output: &'a mut Vec<u8>,
        budget: StackBudget,
    ) -> Result<(), Error> {
        Self::encode_observed(graph, target, output, budget, None)
    }
    /// Append a name and observe the existing dictionary work.
    ///
    /// # Errors
    /// Returns an encoding diagnostic or a checked storage/stack limit.
    pub fn encode_observed(
        graph: &'a Graph,
        target: Target,
        output: &'a mut Vec<u8>,
        budget: StackBudget,
        stats: Option<&'a mut Statistics>,
    ) -> Result<(), Error> {
        let marker = 0u8;
        let start = output.len();
        let mut encoder = Self {
            graph,
            slots: vec![None; graph.len()],
            entries: 0,
            output,
            budget,
            stack_base: std::ptr::from_ref(&marker).addr(),
            stats,
        };
        encoder.target(target)?;
        if let Some(stats) = &mut encoder.stats {
            stats.output_bytes += encoder.output.len() - start;
        }
        Ok(())
    }
    fn put(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
    }
    fn number(&mut self, number: u64) {
        self.put(number.to_string().as_bytes());
    }
    fn signed(&mut self, number: i64) {
        if number < 0 {
            self.put(b"n");
        }
        self.number(number.unsigned_abs());
    }
    fn source_name(&mut self, id: NodeId) {
        let text = self.graph.text(id);
        self.put(text.len().to_string().as_bytes());
        self.put(text);
    }
    fn tags(&mut self, id: NodeId) {
        for tag in self.graph.items(id) {
            self.put(b"B");
            self.source_name(*tag);
        }
    }
    fn substitute(&mut self, id: NodeId) -> bool {
        if let Some(stats) = &mut self.stats {
            stats.substitution_lookups += 1;
        }
        let Some(index) = self.slots[id.index()] else {
            return false;
        };
        if let Some(stats) = &mut self.stats {
            stats.substitution_hits += 1;
        }
        self.put(b"S");
        if index != 0 {
            let mut number = index - 1;
            let mut digits = Vec::new();
            loop {
                let digit = u8::try_from(number % 36).unwrap_or(0);
                digits.push(if digit < 10 {
                    b'0' + digit
                } else {
                    b'A' + digit - 10
                });
                number /= 36;
                if number == 0 {
                    break;
                }
            }
            digits.reverse();
            self.put(&digits);
        }
        self.put(b"_");
        true
    }
    fn record(&mut self, id: NodeId) -> Result<(), Error> {
        if self.slots[id.index()].is_none() {
            self.slots[id.index()] = Some(self.entries);
            self.entries = self.entries.checked_add(1).ok_or(Error::StorageLimit)?;
            if let Some(stats) = &mut self.stats {
                stats.substitution_entries += 1;
            }
        }
        Ok(())
    }
    fn check_stack(&self) -> Result<(), Error> {
        let marker = 0u8;
        if self
            .budget
            .exceeded(self.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(Error::diagnostic("ABI name nested too deeply"));
        }
        Ok(())
    }
    fn qualifiers(&mut self, qualifiers: Qualifiers) {
        if qualifiers.contains(Qualifiers::RESTRICT) {
            self.put(b"r");
        }
        if qualifiers.contains(Qualifiers::VOLATILE) {
            self.put(b"V");
        }
        if qualifiers.contains(Qualifiers::CONST) {
            self.put(b"K");
        }
    }
    fn reference_qualifier(&mut self, qualifiers: Qualifiers) {
        if qualifiers.contains(Qualifiers::LVALUE) {
            self.put(b"R");
        } else if qualifiers.contains(Qualifiers::RVALUE) {
            self.put(b"O");
        }
    }
    fn encode_type(&mut self, mut id: NodeId) -> Result<(), Error> {
        self.check_stack()?;
        let mut chain = Vec::new();
        loop {
            let node = self.graph.get(id);
            match node {
                Node::Builtin(builtin) if !builtin.substitutable() => {
                    self.put(builtin.entry().0);
                    break;
                }
                Node::Abbreviation(kind) if self.graph.items(id).is_empty() => {
                    self.put(kind.code());
                    break;
                }
                _ => {}
            }
            if self.substitute(id) {
                break;
            }
            let next = match node {
                Node::Modified { modifier, operand } => {
                    match modifier {
                        Modifier::Pointer => self.put(b"P"),
                        Modifier::LvalueReference => self.put(b"R"),
                        Modifier::RvalueReference => self.put(b"O"),
                        Modifier::PackExpansion => self.put(b"Dp"),
                        Modifier::Qualified(cv) => self.qualifiers(cv),
                        Modifier::Vendor(name) => {
                            self.put(b"U");
                            self.source_name(name);
                        }
                        Modifier::ExpressionArray(expression) => {
                            self.put(b"A");
                            self.expression(expression)?;
                            self.put(b"_");
                        }
                        Modifier::Array(bound) => {
                            self.put(b"A");
                            self.number(bound);
                            self.put(b"_");
                        }
                    }
                    Some(operand)
                }
                Node::MemberPointer { owner, member } => {
                    self.put(b"M");
                    self.encode_type(owner)?;
                    Some(member)
                }
                _ => None,
            };
            let Some(next) = next else {
                self.type_node(id)?;
                break;
            };
            chain.push(id);
            id = next;
        }
        for id in chain.into_iter().rev() {
            self.record(id)?;
        }
        Ok(())
    }
    fn type_node(&mut self, id: NodeId) -> Result<(), Error> {
        match self.graph.get(id) {
            Node::Builtin(builtin) => self.put(builtin.entry().0),
            Node::Abbreviation(kind) => {
                self.put(kind.code());
                self.tags(id);
            }
            Node::Name { .. } | Node::Template(_) => self.name(id, Qualifiers::default())?,
            Node::TemplateParam { .. } => return self.prefix(Some(id)),
            Node::Decltype {
                expression,
                id_expression,
            } => {
                self.put(if id_expression { b"Dt" } else { b"DT" });
                self.expression(expression)?;
                self.put(b"E");
            }
            Node::VendorType(name) => {
                self.put(b"u");
                self.source_name(name);
                if !self.graph.items(id).is_empty() {
                    self.template_args(id)?;
                }
            }
            Node::FunctionType {
                result,
                qualifiers,
                variadic,
                ..
            } => {
                self.qualifiers(qualifiers);
                self.put(b"F");
                self.encode_type(result)?;
                self.parameters(id, variadic)?;
                self.reference_qualifier(qualifiers);
                self.put(b"E");
            }
            _ => return Err(Error::diagnostic("ABI node is not a type")),
        }
        self.record(id)
    }
    fn name(&mut self, id: NodeId, qualifiers: Qualifiers) -> Result<(), Error> {
        let node = self.graph.get(id);
        if let Node::Abbreviation(kind) = node {
            self.put(kind.code());
            self.tags(id);
            return Ok(());
        }
        let final_id = if let Node::Template(template) = node {
            template
        } else {
            id
        };
        let Node::Name { parent, .. } = self.graph.get(final_id) else {
            if matches!(node, Node::Template(_)) {
                self.prefix(Some(final_id))?;
                return self.template_args(id);
            }
            return Err(Error::diagnostic("ABI name has no final name component"));
        };
        if parent.is_none_or(|p| self.graph.get(p) == Node::Std) {
            if !qualifiers.is_empty() {
                return Err(Error::diagnostic(
                    "function qualifiers need a member function name",
                ));
            }
            if matches!(node, Node::Template(_)) {
                self.prefix(Some(final_id))?;
                return self.template_args(id);
            }
            if parent.is_some() {
                self.put(b"St");
            }
            return self.unqualified(final_id);
        }
        // Find the entity directly in the function without copying its chain.
        // A known prefix already represents that scope in an ordinary nested name.
        let mut root = parent;
        let mut local = final_id;
        while let Some(component) = root {
            let component = if let Node::Template(template) = self.graph.get(component) {
                template
            } else {
                component
            };
            let Node::Name { parent, .. } = self.graph.get(component) else {
                break;
            };
            local = component;
            root = parent;
        }
        if root.is_some_and(|root| self.is_function_scope(root))
            && (local == final_id || !self.chain_has_known_prefix(id))
        {
            return self.local_name(id, local, qualifiers);
        }
        self.nested_name(id, qualifiers)
    }
    fn is_function_scope(&self, id: NodeId) -> bool {
        matches!(
            self.graph.get(id),
            Node::FunctionScope { .. } | Node::RawFunctionScope(_)
        )
    }
    fn chain_has_known_prefix(&self, id: NodeId) -> bool {
        let mut prefix = match self.graph.get(id) {
            Node::Name { parent, .. } => parent,
            Node::Template(template) => Some(template),
            _ => None,
        };
        while let Some(id) = prefix {
            prefix = match self.graph.get(id) {
                Node::Name { parent, .. } => parent,
                Node::Template(template) => Some(template),
                _ => return false,
            };
            if self.slots[id.index()].is_some() {
                return true;
            }
        }
        false
    }
    fn local_name(
        &mut self,
        id: NodeId,
        local: NodeId,
        qualifiers: Qualifiers,
    ) -> Result<(), Error> {
        let Node::Name {
            parent: Some(scope),
            occurrence,
            ..
        } = self.graph.get(local)
        else {
            return Err(Error::diagnostic("invalid ABI local name"));
        };
        self.context(scope)?;
        let final_id = if let Node::Template(template) = self.graph.get(id) {
            template
        } else {
            id
        };
        if final_id == local {
            if !qualifiers.is_empty() {
                return Err(Error::diagnostic(
                    "function qualifiers need a member function name",
                ));
            }
            if matches!(self.graph.get(id), Node::Template(_)) {
                self.prefix(Some(final_id))?;
                self.template_args(id)?;
            } else {
                self.unqualified(local)?;
            }
        } else {
            self.nested_name(id, qualifiers)?;
        }
        if occurrence != 0 {
            let number = occurrence - 1;
            self.put(if number < 10 { b"_" } else { b"__" });
            self.number(u64::from(number));
            if number >= 10 {
                self.put(b"_");
            }
        }
        Ok(())
    }
    fn context(&mut self, scope: NodeId) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.get(scope) {
            Node::RawFunctionScope(fragment) => self.put(self.graph.text(fragment)),
            Node::FunctionScope {
                encoding,
                default_argument,
            } => {
                self.put(b"Z");
                self.entity(encoding)?;
                self.put(b"E");
                if default_argument != 0 {
                    self.put(b"d");
                    if default_argument > 1 {
                        self.number(u64::from(default_argument - 2));
                    }
                    self.put(b"_");
                }
            }
            _ => return Err(Error::diagnostic("invalid ABI local context")),
        }
        Ok(())
    }
    fn nested_name(&mut self, id: NodeId, qualifiers: Qualifiers) -> Result<(), Error> {
        self.put(b"N");
        self.qualifiers(qualifiers);
        self.reference_qualifier(qualifiers);
        match self.graph.get(id) {
            Node::Template(template) => {
                self.prefix(Some(template))?;
                self.template_args(id)?;
            }
            Node::Name { parent, .. } => {
                self.prefix(parent)?;
                self.unqualified(id)?;
            }
            _ => return Err(Error::diagnostic("ABI name has no final name component")),
        }
        self.put(b"E");
        Ok(())
    }
    fn prefix(&mut self, mut id: Option<NodeId>) -> Result<(), Error> {
        self.check_stack()?;
        let mut chain = Vec::new();
        while let Some(current) = id {
            match self.graph.get(current) {
                Node::Name { parent, .. } if self.slots[current.index()].is_none() => {
                    chain.push(current);
                    // The context was emitted by local_name; it is not a prefix candidate.
                    id = parent.filter(|parent| !self.is_function_scope(*parent));
                }
                Node::Template(template) if self.slots[current.index()].is_none() => {
                    chain.push(current);
                    id = Some(template);
                }
                _ => break,
            }
        }
        if let Some(id) = id {
            self.prefix_root(id)?;
        }
        for id in chain.into_iter().rev() {
            if matches!(self.graph.get(id), Node::Name { .. }) {
                self.unqualified(id)?;
            } else {
                self.template_args(id)?;
            }
            self.record(id)?;
        }
        Ok(())
    }
    fn prefix_root(&mut self, id: NodeId) -> Result<(), Error> {
        match self.graph.get(id) {
            Node::Name { .. } | Node::Template(_) => {
                self.substitute(id);
            }
            Node::Std => self.put(b"St"),
            Node::Abbreviation(kind) => {
                if self.graph.items(id).is_empty() {
                    self.put(kind.code());
                } else if !self.substitute(id) {
                    self.put(kind.code());
                    self.tags(id);
                    self.record(id)?;
                }
            }
            Node::TemplateParam {
                index,
                substitutable,
            } => {
                if self.substitute(id) {
                    return Ok(());
                }
                self.put(b"T");
                if index != 0 {
                    self.number(u64::from(index - 1));
                }
                self.put(b"_");
                if substitutable {
                    self.record(id)?;
                }
            }
            Node::VendorType(_) | Node::Decltype { .. } => self.encode_type(id)?,
            _ => return Err(Error::diagnostic("invalid ABI name prefix")),
        }
        Ok(())
    }
    fn unqualified(&mut self, id: NodeId) -> Result<(), Error> {
        let Node::Name {
            terminal, internal, ..
        } = self.graph.get(id)
        else {
            return Err(Error::diagnostic("invalid ABI unqualified name"));
        };
        if internal {
            self.put(b"L");
        }
        match self.graph.get(terminal) {
            Node::Text => self.source_name(terminal),
            Node::Terminal { kind, inherited } => {
                if let Some(base) = inherited {
                    self.put(if kind == super::vocabulary::Terminal::ConstructorBase {
                        b"CI2"
                    } else {
                        b"CI1"
                    });
                    if matches!(self.graph.get(base), Node::Name { .. } | Node::Template(_)) {
                        self.name(base, Qualifiers::default())?;
                    } else {
                        self.encode_type(base)?;
                    }
                } else {
                    self.put(kind.code());
                }
            }
            Node::Conversion(ty) => {
                self.put(b"cv");
                self.encode_type(ty)?;
            }
            Node::LiteralOperator(suffix) => {
                self.put(b"li");
                self.source_name(suffix);
            }
            Node::ClosureName { number, variadic } => {
                self.put(b"Ul");
                self.parameters(terminal, variadic)?;
                self.put(b"E");
                if number != 0 {
                    self.number(u64::from(number - 1));
                }
                self.put(b"_");
            }
            _ => return Err(Error::diagnostic("invalid ABI unqualified name")),
        }
        self.tags(id);
        Ok(())
    }
    fn template_args(&mut self, id: NodeId) -> Result<(), Error> {
        self.put(b"I");
        for argument in self.graph.items(id) {
            self.argument(*argument)?;
        }
        self.put(b"E");
        Ok(())
    }
    fn argument(&mut self, id: NodeId) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.get(id) {
            Node::ExpressionArgument(expression) => {
                self.put(b"X");
                self.expression(expression)?;
                self.put(b"E");
            }
            Node::Literal { ty, bits } => {
                self.put(b"L");
                self.encode_type(ty)?;
                if matches!(self.graph.get(ty), Node::Builtin(b) if b.entry().2) {
                    self.number(bits);
                } else {
                    self.signed(i64::from_ne_bytes(bits.to_ne_bytes()));
                }
                self.put(b"E");
            }
            Node::TypedArgument {
                declaration,
                argument,
            } => {
                let Node::NonTypeParameter(ty) = self.graph.get(declaration) else {
                    return Err(Error::diagnostic(
                        "invalid ABI template parameter declaration",
                    ));
                };
                self.put(b"Tn");
                self.encode_type(ty)?;
                self.argument(argument)?;
            }
            Node::ArgumentPack => {
                self.put(b"J");
                for argument in self.graph.items(id) {
                    self.argument(*argument)?;
                }
                self.put(b"E");
            }
            Node::EntityArgument { entity, address } => {
                if address {
                    self.put(b"Xad");
                }
                self.put(b"L");
                self.entity_reference(entity)?;
                self.put(b"E");
                if address {
                    self.put(b"E");
                }
            }
            _ => self.encode_type(id)?,
        }
        Ok(())
    }
    // Expressions are never dictionary entries. Their type operands use the
    // same dictionary as the enclosing name, in source operand order.
    fn expression(&mut self, id: NodeId) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.get(id) {
            Node::TemplateParam { index, .. } => {
                self.put(b"T");
                if index != 0 {
                    self.number(u64::from(index - 1));
                }
                self.put(b"_");
            }
            Node::FunctionParam(index) => {
                self.put(b"fp");
                if index != 0 {
                    self.number(u64::from(index - 1));
                }
                self.put(b"_");
            }
            Node::Literal { .. } => self.argument(id)?,
            Node::Operation(operation) => {
                self.put(operation.code());
                for item in self.graph.items(id) {
                    self.expression(*item)?;
                }
            }
            Node::Call => {
                self.put(b"cl");
                for item in self.graph.items(id) {
                    self.expression(*item)?;
                }
                self.put(b"E");
            }
            Node::Cast {
                operation,
                ty,
                operand,
            } => {
                self.put(operation.code());
                self.encode_type(ty)?;
                self.expression(operand)?;
            }
            Node::UnresolvedName(name) => {
                self.source_name(name);
                if !self.graph.items(id).is_empty() {
                    self.template_args(id)?;
                }
            }
            Node::VendorExpression(name) => {
                self.put(b"u");
                self.source_name(name);
                for item in self.graph.items(id) {
                    self.argument(*item)?;
                }
                self.put(b"E");
            }
            Node::SizeofType(ty) => {
                self.put(b"st");
                self.encode_type(ty)?;
            }
            Node::ScopedMember {
                owner,
                member,
                close_qualifier,
            } => {
                self.put(b"sr");
                self.encode_type(owner)?;
                if close_qualifier {
                    self.put(b"E");
                }
                self.source_name(member);
                if !self.graph.items(id).is_empty() {
                    self.template_args(id)?;
                }
            }
            Node::MemberAccess {
                operation,
                object,
                member,
            } => {
                self.put(operation.code());
                self.expression(object)?;
                self.source_name(member);
                if !self.graph.items(id).is_empty() {
                    self.template_args(id)?;
                }
            }
            Node::EntityReference(entity) => {
                self.put(b"L");
                self.entity_reference(entity)?;
                self.put(b"E");
            }
            _ => return Err(Error::diagnostic("ABI node is not an expression")),
        }
        Ok(())
    }
    fn entity_reference(&mut self, entity: NodeId) -> Result<(), Error> {
        if let Some(stats) = &mut self.stats {
            stats.entity_references += 1;
        }
        if let Node::ExternalSymbol(symbol) = self.graph.get(entity) {
            self.put(self.graph.text(symbol));
        } else {
            self.put(b"_Z");
            self.entity(entity)?;
        }
        Ok(())
    }
    fn parameters(&mut self, id: NodeId, variadic: bool) -> Result<(), Error> {
        for parameter in self.graph.items(id) {
            self.encode_type(*parameter)?;
        }
        if self.graph.items(id).is_empty() && !variadic {
            self.put(b"v");
        }
        if variadic {
            self.put(b"z");
        }
        Ok(())
    }
    fn entity(&mut self, id: NodeId) -> Result<(), Error> {
        if let Node::Function {
            name,
            result,
            qualifiers,
            variadic,
        } = self.graph.get(id)
        {
            self.name(name, qualifiers)?;
            if let (Some(result), Node::Template(template)) = (result, self.graph.get(name)) {
                let terminal = match self.graph.get(template) {
                    Node::Name { terminal, .. } => Some(self.graph.get(terminal)),
                    _ => None,
                };
                if !matches!(terminal, Some(Node::Terminal { kind, .. }) if kind.constructor_or_destructor())
                    && !matches!(terminal, Some(Node::Conversion(_)))
                {
                    self.encode_type(result)?;
                }
            }
            return self.parameters(id, variadic);
        }
        if !matches!(self.graph.get(id), Node::Name { .. } | Node::Template(_)) {
            return Err(Error::diagnostic(
                "ABI entity is neither a function nor a data name",
            ));
        }
        self.name(id, Qualifiers::default())
    }
    fn call_offset(&mut self, offset: CallOffset) {
        self.put(if offset.vcall.is_some() { b"v" } else { b"h" });
        self.signed(offset.offset);
        self.put(b"_");
        if let Some(vcall) = offset.vcall {
            self.signed(vcall);
            self.put(b"_");
        }
    }
    fn target(&mut self, target: Target) -> Result<(), Error> {
        match target {
            Target::Type(id) => self.encode_type(id)?,
            Target::Entity(id) => {
                self.put(b"_Z");
                self.entity(id)?;
            }
            Target::CName(id) => self.put(self.graph.text(id)),
            Target::Typeinfo(id)
            | Target::TypeinfoName(id)
            | Target::Vtable(id)
            | Target::Vtt(id) => {
                self.put(match target {
                    Target::Typeinfo(_) => b"_ZTI",
                    Target::TypeinfoName(_) => b"_ZTS",
                    Target::Vtable(_) => b"_ZTV",
                    _ => b"_ZTT",
                });
                self.encode_type(id)?;
            }
            Target::TlsWrapper(id) => {
                self.put(b"_ZTW");
                self.name(id, Qualifiers::default())?;
            }
            Target::ConstructionVtable {
                derived,
                offset,
                base,
            } => {
                self.put(b"_ZTC");
                self.encode_type(derived)?;
                self.signed(offset);
                self.put(b"_");
                self.encode_type(base)?;
            }
            Target::Thunk {
                function,
                this,
                result,
            } => {
                self.put(b"_ZT");
                if result.is_some() {
                    self.put(b"c");
                }
                self.call_offset(this);
                if let Some(result) = result {
                    self.call_offset(result);
                }
                self.entity(function)?;
            }
        }
        Ok(())
    }
}
