//! Fact type constructors and integral arguments, translated from `abi_fact_definitions.cpp`. See NOTICE.
use super::{
    Error,
    facts::{CaseReader, DefinitionKind},
    graph::{Abbreviation, Modifier, Node, NodeId, Qualifiers},
    vocabulary::Builtin,
};

enum Wrapper {
    Modifier(Modifier),
    Transform(NodeId),
}
impl CaseReader<'_, '_> {
    pub(super) fn type_spec(&mut self, line: usize, mut words: &[&[u8]]) -> Result<NodeId, Error> {
        let mut wrappers = Vec::new();
        while words.len() >= 2 {
            if let Some(modifier) = simple_modifier(words[0]) {
                wrappers.push(Wrapper::Modifier(modifier));
                words = &words[1..];
            } else if words.len() >= 3 && matches!(words[0], b"vendor" | b"builtin-transform") {
                let name = self.graph.string(words[1])?;
                wrappers.push(if words[0] == b"vendor" {
                    Wrapper::Modifier(Modifier::Vendor(name))
                } else {
                    Wrapper::Transform(name)
                });
                words = &words[2..];
            } else {
                break;
            }
        }
        let mut ty = self.type_form(line, words)?;
        for wrapper in wrappers.into_iter().rev() {
            ty = match wrapper {
                Wrapper::Modifier(modifier) => self.graph.modified(modifier, ty)?,
                Wrapper::Transform(name) => self.graph.intern(Node::VendorType(name), &[ty])?,
            };
        }
        Ok(ty)
    }
    fn type_form(&mut self, line: usize, words: &[&[u8]]) -> Result<NodeId, Error> {
        let Some(form) = words.first().copied() else {
            return Err(self.error(line, "missing type"));
        };
        if words.len() == 1 {
            if is_type_form(form) {
                return Err(self.word_error(line, "incomplete type form '", form, "'"));
            }
            return self.compact_type(line, form);
        }
        match form {
            b"named" | b"name" if words.len() == 2 => return self.qualified_name(line, words[1]),
            b"template" if words.len() >= 3 => {
                let template = self.qualified_name(line, words[1])?;
                let args = self.argument_list(&words[2..])?;
                return self.graph.intern(Node::Template(template), &args);
            }
            b"std-template" if words.len() >= 5 => {
                let kind = Abbreviation::from_word(words[1]).ok_or_else(|| {
                    self.word_error(line, "unknown standard substitution '", words[1], "'")
                })?;
                let abbreviation = self.graph.intern(Node::Abbreviation(kind), &[])?;
                let args = self.argument_list(&words[4..])?;
                if self.flag(line, words[2])? {
                    return Ok(abbreviation);
                }
                return self.graph.intern(Node::Template(abbreviation), &args);
            }
            b"template-param" | b"template-param-subst" if words.len() == 2 => {
                let index = self.index_value(line, words[1])?;
                return self.graph.intern(
                    Node::TemplateParam {
                        index,
                        substitutable: form == b"template-param-subst",
                    },
                    &[],
                );
            }
            b"template-param-template" if words.len() >= 3 => {
                let index = self.index_value(line, words[1])?;
                let template = self.graph.intern(
                    Node::TemplateParam {
                        index,
                        substitutable: true,
                    },
                    &[],
                )?;
                let args = self.argument_list(&words[2..])?;
                return self.graph.intern(Node::Template(template), &args);
            }
            b"member" if words.len() == 3 => {
                let parent = self.compact_type(line, words[1])?;
                let terminal = self.graph.string(words[2])?;
                return self.graph.name(Some(parent), terminal, &[]);
            }
            b"member-template" if words.len() >= 4 => {
                let parent = self.compact_type(line, words[1])?;
                let terminal = self.graph.string(words[2])?;
                let template = self.graph.name(Some(parent), terminal, &[])?;
                let args = self.argument_list(&words[3..])?;
                return self.graph.intern(Node::Template(template), &args);
            }
            b"namespace-lambda" => {
                let parent = self.namespace_path(line, &words[2..])?;
                let terminal = self.graph.string(words[1])?;
                return self.graph.name(parent, terminal, &[]);
            }
            b"tagged" if words.len() >= 3 => {
                let ty = self.compact_type(line, words[1])?;
                let named = if let Node::Template(template) = self.graph.get(ty) {
                    template
                } else {
                    ty
                };
                if !matches!(
                    self.graph.get(named),
                    Node::Name { .. } | Node::Abbreviation(_)
                ) {
                    return Err(
                        self.error(line, "tagged type needs a named or class-template type")
                    );
                }
                let tags = words[2..]
                    .iter()
                    .map(|word| self.graph.string(word))
                    .collect::<Result<Vec<_>, _>>()?;
                return self.graph.tagged(ty, &tags);
            }
            b"function-type" | b"function-type-variadic" => {
                let result = self.compact_type(line, words[1])?;
                let mut params = Vec::new();
                for word in &words[2..] {
                    params.push(self.compact_type(line, word)?);
                }
                self.graph.drop_void(&mut params);
                return self.graph.intern(
                    Node::FunctionType {
                        result,
                        owner: None,
                        qualifiers: Qualifiers::default(),
                        variadic: form == b"function-type-variadic",
                    },
                    &params,
                );
            }
            b"member-pointer" if words.len() == 3 => {
                let owner = self.compact_type(line, words[1])?;
                let member = self.compact_type(line, words[2])?;
                return self.graph.member_pointer(owner, member);
            }
            b"decltype" | b"decltype-id" | b"lambda-closure" | b"local-type" => {
                return Err(Error::NotImplemented("dependent and local ABI types"));
            }
            _ => {}
        }
        Err(self.word_error(line, "unknown or malformed type form '", form, "'"))
    }
    pub(super) fn compact_type(&mut self, line: usize, mut word: &[u8]) -> Result<NodeId, Error> {
        self.check_type_stack(line)?;
        let mut modifiers = Vec::new();
        while let Some(colon) = word.iter().position(|c| *c == b':') {
            let prefix = &word[..colon];
            if let Some(modifier) = simple_modifier(prefix) {
                modifiers.push(modifier);
                word = &word[colon + 1..];
            } else if prefix == b"array" {
                word = &word[colon + 1..];
                let end = word
                    .iter()
                    .position(|c| *c == b':')
                    .filter(|n| *n != 0)
                    .ok_or_else(|| self.error(line, "missing array bound"))?;
                let bound = &word[..end];
                if !bound[0].is_ascii_digit() {
                    self.reference(bound, DefinitionKind::Expression, "array bound expression")?;
                    return Err(Error::NotImplemented("expression ABI array bounds"));
                }
                let value = self.integer_value(line, bound)?;
                modifiers.push(Modifier::Array(u64::from_ne_bytes(value.to_ne_bytes())));
                word = &word[end + 1..];
            } else {
                break;
            }
        }
        if word.is_empty() {
            return Err(self.error(line, "missing ABI type after prefix modifier"));
        }
        let mut ty = if let Some(path) = word.strip_prefix(b"named:") {
            self.qualified_name(line, path)?
        } else if let Some(member) = word.strip_prefix(b"memberptr:") {
            self.compact_member_pointer(line, member)?
        } else if self.defined_as(word, DefinitionKind::Type) {
            self.reference(word, DefinitionKind::Type, "type")?
        } else if let Some(builtin) = Builtin::from_word(word) {
            self.graph.intern(Node::Builtin(builtin), &[])?
        } else {
            self.qualified_name(line, word)?
        };
        for modifier in modifiers.into_iter().rev() {
            ty = self.graph.modified(modifier, ty)?;
        }
        Ok(ty)
    }
    fn compact_member_pointer(&mut self, line: usize, word: &[u8]) -> Result<NodeId, Error> {
        let mut split = 0;
        while split < word.len() {
            if word[split] == b':' {
                if word.get(split + 1) == Some(&b':') {
                    split += 2;
                    continue;
                }
                break;
            }
            split += 1;
        }
        if split == 0 || split + 1 >= word.len() {
            return Err(self.error(line, "invalid compact member-pointer type"));
        }
        let owner = if self.defined_as(&word[..split], DefinitionKind::Type) {
            self.reference(&word[..split], DefinitionKind::Type, "type")?
        } else {
            self.qualified_name(line, &word[..split])?
        };
        let member = self.compact_type(line, &word[split + 1..])?;
        self.graph.member_pointer(owner, member)
    }
    pub(super) fn namespace_path(
        &mut self,
        line: usize,
        words: &[&[u8]],
    ) -> Result<Option<NodeId>, Error> {
        let mut parent = None;
        for (index, word) in words.iter().enumerate() {
            if word.contains(&b':') {
                return Err(self.word_error(
                    line,
                    "stray ':' in ABI namespace qualifier '",
                    word,
                    "'",
                ));
            }
            parent = Some(if index == 0 && *word == b"std" {
                self.graph.intern(Node::Std, &[])?
            } else {
                let terminal = self.graph.string(word)?;
                self.graph.name(parent, terminal, &[])?
            });
        }
        Ok(parent)
    }
    pub(super) fn argument_list(&mut self, words: &[&[u8]]) -> Result<Vec<NodeId>, Error> {
        words
            .iter()
            .map(|word| self.reference(word, DefinitionKind::Argument, "template argument"))
            .collect()
    }
    pub(super) fn argument_definition(
        &mut self,
        line: usize,
        words: &[&[u8]],
    ) -> Result<NodeId, Error> {
        match words[0] {
            b"type" => return self.type_spec(line, &words[1..]),
            b"value" if words.len() == 3 => {
                let ty = self.compact_type(line, words[1])?;
                return self.typed_literal(line, ty, words[2]);
            }
            b"dependent-value" if words.len() == 4 => {
                let ty = self.compact_type(line, words[1])?;
                let literal_ty = self.compact_type(line, words[2])?;
                let argument = self.typed_literal(line, literal_ty, words[3])?;
                return self.graph.intern(Node::TypedArgument { ty, argument }, &[]);
            }
            b"template-entity" if words.len() == 2 => return self.qualified_name(line, words[1]),
            b"member-template-entity" if words.len() == 4 => {
                let owner = self.compact_type(line, words[1])?;
                let terminal = self.graph.string(words[2])?;
                return self.graph.name(Some(owner), terminal, &[]);
            }
            b"template-param-template" if words.len() == 2 => {
                let index = self.index_value(line, words[1])?;
                return self.graph.intern(
                    Node::TemplateParam {
                        index,
                        substitutable: false,
                    },
                    &[],
                );
            }
            b"pack" => {
                let args = self.argument_list(&words[1..])?;
                return self.graph.intern(Node::ArgumentPack, &args);
            }
            b"entity-address" | b"entity-reference" if words.len() == 2 => {
                let entity = self.reference(words[1], DefinitionKind::Entity, "entity")?;
                return self.graph.intern(
                    Node::EntityArgument {
                        entity,
                        address: words[0] == b"entity-address",
                    },
                    &[],
                );
            }
            b"member-external-address" => return self.member_address(line, &words[1..]),
            b"expression" => return Err(Error::NotImplemented("ABI expression arguments")),
            _ => {}
        }
        Err(self.word_error(
            line,
            "unknown or malformed template argument form '",
            words[0],
            "'",
        ))
    }
    fn typed_literal(&mut self, line: usize, ty: NodeId, word: &[u8]) -> Result<NodeId, Error> {
        let mut value = self.integer_value(line, word)?;
        if let Node::Builtin(builtin) = self.graph.get(ty) {
            let (_, bits, unsigned) = builtin.entry();
            if bits == 0 {
                return Err(self.error(line, "value needs an integral type"));
            }
            if builtin == Builtin::Bool && value != 0 && value != 1 {
                return Err(self.error(line, "bool ABI value must be 0 or 1"));
            }
            let negative_input = word.starts_with(b"-");
            if unsigned {
                if bits < 64 {
                    let mask = (1u64 << bits) - 1;
                    let pattern = u64::from_ne_bytes(value.to_ne_bytes());
                    if !negative_input && pattern > mask {
                        return Err(self.word_error(
                            line,
                            "unsigned ABI value out of range '",
                            word,
                            "'",
                        ));
                    }
                    value = i64::from_ne_bytes((pattern & mask).to_ne_bytes());
                }
            } else {
                let out_of_range = (!negative_input && value < 0)
                    || (bits < 64 && {
                        let limit = 1i64 << (bits - 1);
                        value < -limit || value >= limit
                    });
                if out_of_range {
                    return Err(self.word_error(
                        line,
                        "signed ABI value out of range '",
                        word,
                        "'",
                    ));
                }
            }
        }
        self.graph.intern(
            Node::Literal {
                ty,
                bits: u64::from_ne_bytes(value.to_ne_bytes()),
            },
            &[],
        )
    }
}
fn simple_modifier(word: &[u8]) -> Option<Modifier> {
    Some(match word {
        b"ptr" => Modifier::Pointer,
        b"ref" => Modifier::LvalueReference,
        b"rref" => Modifier::RvalueReference,
        b"pack" => Modifier::PackExpansion,
        b"const" => Modifier::Qualified(Qualifiers::CONST),
        b"volatile" => Modifier::Qualified(Qualifiers::VOLATILE),
        _ => return None,
    })
}
fn is_type_form(word: &[u8]) -> bool {
    simple_modifier(word).is_some()
        || matches!(
            word,
            b"vendor"
                | b"builtin-transform"
                | b"named"
                | b"name"
                | b"template"
                | b"std-template"
                | b"template-param"
                | b"template-param-subst"
                | b"template-param-template"
                | b"member"
                | b"member-template"
                | b"decltype"
                | b"decltype-id"
                | b"lambda-closure"
                | b"local-type"
                | b"namespace-lambda"
                | b"tagged"
                | b"function-type"
                | b"function-type-variadic"
                | b"member-pointer"
        )
}
