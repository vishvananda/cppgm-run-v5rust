//! Function facts, structured components and terminal selection. See NOTICE.
use super::{
    Error,
    facts::{CaseReader, DefinitionKind},
    graph::{Abbreviation, Node, NodeId, Qualifiers},
    vocabulary::Terminal,
};

#[derive(Clone, Copy, Default)]
enum Component {
    #[default]
    Missing,
    Placeholder,
    Node(NodeId),
}
#[derive(Clone, Copy)]
enum TerminalFact {
    Fixed(Terminal),
    Node(NodeId),
}
#[derive(Default)]
pub(super) struct FunctionFacts {
    structured: bool,
    scope: Option<NodeId>,
    final_component: Component,
    terminal: Option<TerminalFact>,
    terminal_records: u32,
    context_records: u32,
    component_tags: Vec<NodeId>,
    tags: Vec<NodeId>,
    args: Vec<NodeId>,
    result: Option<NodeId>,
    params: Vec<NodeId>,
    variadic: bool,
    qualifiers: Qualifiers,
    generated_lambda: bool,
}
impl CaseReader<'_, '_> {
    pub(super) fn entity_definition(
        &mut self,
        line: usize,
        words: &[&[u8]],
    ) -> Result<NodeId, Error> {
        match words[0] {
            b"variable" | b"internal-variable" if words.len() == 2 => {
                let id = self.qualified_name(line, words[1])?;
                if words[0] == b"variable" {
                    return Ok(id);
                }
                let Node::Name {
                    parent,
                    terminal,
                    occurrence,
                    ..
                } = self.graph.get(id)
                else {
                    return Err(Error::StorageLimit);
                };
                let tags = self.graph.items(id).to_vec();
                return self.graph.intern(
                    Node::Name {
                        parent,
                        terminal,
                        internal: true,
                        occurrence,
                    },
                    &tags,
                );
            }
            b"symbol" if words.len() == 2 => {
                let symbol = self.graph.string(words[1])?;
                return self.graph.intern(Node::ExternalSymbol(symbol), &[]);
            }
            b"function" if words.len() >= 2 => {
                if words[1] == b"encoding" {
                    return Err(self.error(line, "a function entity needs a path"));
                }
                let function = self.function_target(line, &words[1..])?;
                return self.finish_function(line, function);
            }
            _ => {}
        }
        Err(self.word_error(line, "unknown or malformed entity form '", words[0], "'"))
    }
    pub(super) fn member_address(&mut self, line: usize, words: &[&[u8]]) -> Result<NodeId, Error> {
        if words.len() < 9 || words[0].is_empty() {
            return Err(self.error(line, "member external address has invalid operands"));
        }
        let owner = self.compact_type(line, words[1])?;
        let terminal = self.graph.string(words[2])?;
        let member = self.graph.name(Some(owner), terminal, &[])?;
        if !self.flag(line, words[3])? {
            if words.len() != 9 {
                return Err(self.error(line, "member data address has parameter types"));
            }
            return self.graph.intern(
                Node::EntityArgument {
                    entity: member,
                    address: true,
                },
                &[],
            );
        }
        let mut qualifiers = Qualifiers::default();
        for (word, qualifier) in words[4..8].iter().zip([
            Qualifiers::CONST,
            Qualifiers::VOLATILE,
            Qualifiers::LVALUE,
            Qualifiers::RVALUE,
        ]) {
            if self.flag(line, word)? {
                qualifiers = qualifiers.union(qualifier);
            }
        }
        if qualifiers.contains(Qualifiers::LVALUE) && qualifiers.contains(Qualifiers::RVALUE) {
            return Err(self.error(
                line,
                "member external entity has conflicting ref qualifiers",
            ));
        }
        let variadic = self.flag(line, words[8])?;
        let mut params = Vec::new();
        for word in &words[9..] {
            params.push(self.compact_type(line, word)?);
        }
        self.graph.drop_void(&mut params);
        let function = self.graph.intern(
            Node::Function {
                name: member,
                result: None,
                qualifiers,
                variadic,
            },
            &params,
        )?;
        self.graph.intern(
            Node::EntityArgument {
                entity: function,
                address: true,
            },
            &[],
        )
    }
    pub(super) fn function_target(
        &mut self,
        line: usize,
        words: &[&[u8]],
    ) -> Result<FunctionFacts, Error> {
        let Some(form) = words.first().copied() else {
            return Err(self.error(line, "missing function target"));
        };
        let mut function = FunctionFacts::default();
        if form == b"encoding" {
            if words.len() != 1 {
                return Err(self.error(line, "function encoding takes no operands"));
            }
            function.structured = true;
            return Ok(function);
        }
        if matches!(form, b"local" | b"lambda" | b"namespace-lambda") {
            let terminal_word = if form == b"namespace-lambda" { 2 } else { 3 };
            let terminal = words
                .get(terminal_word)
                .and_then(|word| Terminal::from_word(word))
                .ok_or_else(|| {
                    self.word_error(line, "", form, " function target has invalid operands")
                })?;
            function.scope = Some(match form {
                b"local" => {
                    if words.len() != 5 {
                        return Err(self.error(line, "local function target has invalid operands"));
                    }
                    self.local_entity(line, words[1], words[2], words[4])?
                }
                b"lambda" => self.closure_entity(line, words[1], words[2], &words[4..])?,
                _ => {
                    let parent = self.namespace_path(line, &words[3..])?;
                    let name = self.graph.string(words[1])?;
                    self.graph.name(parent, name, &[])?
                }
            });
            function.terminal = Some(TerminalFact::Fixed(terminal));
            function.terminal_records = 1;
            function.context_records = 1;
            return Ok(function);
        }
        let path = form == b"path";
        if path && words.len() < 2 {
            return Err(self.error(line, "function path needs a name"));
        }
        self.set_path(line, words[usize::from(path)], &mut function)?;
        for word in &words[1 + usize::from(path)..] {
            if path && self.defined_as(word, DefinitionKind::Argument) {
                function.args.push(self.reference(
                    word,
                    DefinitionKind::Argument,
                    "template argument",
                )?);
            } else {
                function.params.push(self.compact_type(line, word)?);
            }
        }
        Ok(function)
    }
    fn set_path(
        &mut self,
        line: usize,
        original: &[u8],
        function: &mut FunctionFacts,
    ) -> Result<(), Error> {
        let path = original.strip_prefix(b"::").unwrap_or(original);
        let split = path.windows(2).rposition(|w| w == b"::");
        let last = split.map_or(path, |n| &path[n + 2..]);
        if last.is_empty() {
            return Err(self.error(line, "function path needs a name"));
        }
        if last.contains(&b':') {
            return Err(self.word_error(line, "stray ':' in ABI name '", original, "'"));
        }
        function.scope = if let Some(split) = split {
            Some(if &path[..split] == b"std" {
                self.graph.intern(Node::Std, &[])?
            } else {
                self.qualified_name(line, &path[..split])?
            })
        } else {
            None
        };
        function.final_component = self.component(last)?;
        Ok(())
    }
    fn component(&mut self, word: &[u8]) -> Result<Component, Error> {
        if matches!(word, b"-" | b"operator" | b"") {
            Ok(Component::Placeholder)
        } else {
            Ok(Component::Node(self.graph.string(word)?))
        }
    }
    pub(super) fn function_record(
        &mut self,
        line: usize,
        function: &mut FunctionFacts,
    ) -> Result<(), Error> {
        let words = self.line_words(line).to_vec();
        match words[0] {
            b"param" => function.params.push(self.type_spec(line, &words[1..])?),
            b"result" => {
                if function.result.is_some() {
                    return Err(self.error(line, "multiple ABI function result types"));
                }
                function.result = Some(self.type_spec(line, &words[1..])?);
            }
            b"variadic" => {
                if words.len() != 1 {
                    return Err(self.error(line, "variadic takes no operands"));
                }
                if function.variadic {
                    return Err(self.error(line, "multiple ABI variadic markers"));
                }
                function.variadic = true;
            }
            b"abi-tag" => {
                if words.len() != 2 {
                    return Err(self.error(line, "abi-tag takes one tag"));
                }
                function.tags.push(self.graph.string(words[1])?);
            }
            b"qualifier" | b"function-qualifier" => {
                if words.len() < 2 {
                    return Err(self.error(line, "function qualifier needs at least one qualifier"));
                }
                for word in &words[1..] {
                    let qualifier = match *word {
                        b"const" => Qualifiers::CONST,
                        b"volatile" => Qualifiers::VOLATILE,
                        b"restrict" => Qualifiers::RESTRICT,
                        b"lvalue-ref" => Qualifiers::LVALUE,
                        b"rvalue-ref" => Qualifiers::RVALUE,
                        _ => {
                            return Err(self.word_error(
                                line,
                                "unknown function qualifier '",
                                word,
                                "'",
                            ));
                        }
                    };
                    function.qualifiers = function.qualifiers.union(qualifier);
                }
                if function.qualifiers.contains(Qualifiers::LVALUE)
                    && function.qualifiers.contains(Qualifiers::RVALUE)
                {
                    return Err(self.error(line, "conflicting ABI function reference qualifiers"));
                }
            }
            b"function-template-arg" => {
                if words.len() != 2 {
                    return Err(self.error(line, "function-template-arg takes one id"));
                }
                function.args.push(self.reference(
                    words[1],
                    DefinitionKind::Argument,
                    "template argument",
                )?);
            }
            b"function-template-prefix" => {
                if words.len() != 2 {
                    return Err(self.error(line, "function-template-prefix takes one key"));
                }
            }
            b"operator-terminal" | b"conversion-terminal" | b"terminal" | b"terminal-source" => {
                self.terminal_record(line, &words, function)?;
            }
            b"name-source" | b"name-std" | b"name-template" | b"component-abi-tag" => {
                self.name_record(line, &words, function)?;
            }
            b"local-context" | b"lambda-context" | b"namespace-lambda-context" => {
                if !function.structured {
                    return Err(self.error(
                        line,
                        "structured ABI name records require a function encoding target",
                    ));
                }
                function.context_records += 1;
                if function.context_records > 1 {
                    return Err(self.error(line, "multiple ABI function context kinds"));
                }
                if !matches!(function.final_component, Component::Missing)
                    || function.scope.is_some()
                {
                    return Err(self.error(line, "a local context cannot follow name components"));
                }
                function.scope = Some(match words[0] {
                    b"local-context" => {
                        if words.len() != 4 {
                            return Err(self.error(line, "local-context has invalid operands"));
                        }
                        function.generated_lambda = words[2].starts_with(b"$_")
                            && words[2].len() >= 3
                            && words[2][2..].iter().all(u8::is_ascii_digit);
                        self.local_entity(line, words[1], words[2], words[3])?
                    }
                    b"lambda-context" => {
                        if words.len() < 3 {
                            return Err(self.error(line, "lambda-context has invalid operands"));
                        }
                        self.closure_entity(line, words[1], words[2], &words[3..])?
                    }
                    _ => {
                        if words.len() < 2 {
                            return Err(
                                self.error(line, "namespace-lambda-context needs a source name")
                            );
                        }
                        let parent = self.namespace_path(line, &words[2..])?;
                        let name = self.graph.string(words[1])?;
                        self.graph.name(parent, name, &[])?
                    }
                });
            }
            _ => return Err(self.word_error(line, "unknown ABI function record '", words[0], "'")),
        }
        Ok(())
    }
    fn terminal_record(
        &mut self,
        line: usize,
        words: &[&[u8]],
        function: &mut FunctionFacts,
    ) -> Result<(), Error> {
        function.terminal_records += 1;
        if function.terminal_records > 1 {
            return Err(self.error(line, "multiple ABI function terminals"));
        }
        if words[0] == b"conversion-terminal" {
            if words.len() < 2 {
                return Err(self.error(line, "conversion-terminal needs a type"));
            }
            let ty = self.type_spec(line, &words[1..])?;
            function.terminal = Some(TerminalFact::Node(
                self.graph.intern(Node::Conversion(ty), &[])?,
            ));
            return Ok(());
        }
        if words.len() < 2 {
            return Err(self.word_error(line, "", words[0], " has invalid operands"));
        }
        if words[0] == b"terminal-source" {
            if words.len() != 2 {
                return Err(self.error(line, "terminal-source takes one name"));
            }
            function.terminal = Some(TerminalFact::Node(self.graph.string(words[1])?));
            return Ok(());
        }
        if words[0] == b"operator-terminal" && words[1] == b"literal" {
            if words.len() != 3 {
                return Err(self.error(line, "operator-terminal has invalid operands"));
            }
            let suffix = self.graph.string(words[2])?;
            function.terminal = Some(TerminalFact::Node(
                self.graph.intern(Node::LiteralOperator(suffix), &[])?,
            ));
            return Ok(());
        }
        if words[0] == b"terminal"
            && matches!(
                words[1],
                b"constructor-inherited-complete" | b"constructor-inherited-base"
            )
        {
            if words.len() != 3 {
                return Err(self.error(
                    line,
                    "an inherited constructor terminal needs its base class type",
                ));
            }
            let inherited = self.compact_type(line, words[2])?;
            let kind = if words[1] == b"constructor-inherited-base" {
                Terminal::ConstructorBase
            } else {
                Terminal::ConstructorComplete
            };
            function.terminal = Some(TerminalFact::Node(self.graph.intern(
                Node::Terminal {
                    kind,
                    inherited: Some(inherited),
                },
                &[],
            )?));
            return Ok(());
        }
        let kind = Terminal::from_word(words[1])
            .filter(|_| words.len() == 2)
            .ok_or_else(|| self.word_error(line, "unknown ABI terminal '", words[1], "'"))?;
        if words[0] == b"operator-terminal" && kind.constructor_or_destructor() {
            return Err(self.error(line, "operator-terminal names an operator"));
        }
        function.terminal = Some(TerminalFact::Fixed(kind));
        Ok(())
    }
    fn attach_component(&mut self, line: usize, function: &mut FunctionFacts) -> Result<(), Error> {
        match function.final_component {
            Component::Missing => return Ok(()),
            Component::Placeholder => {
                return Err(self.error(line, "a placeholder name component must be last"));
            }
            Component::Node(mut component) => {
                if self.graph.get(component) == Node::Text {
                    component =
                        self.graph
                            .name(function.scope, component, &function.component_tags)?;
                } else if !function.component_tags.is_empty() {
                    component = self.graph.tagged(component, &function.component_tags)?;
                }
                function.scope = Some(component);
                function.component_tags.clear();
                function.final_component = Component::Missing;
            }
        }
        Ok(())
    }
    fn name_record(
        &mut self,
        line: usize,
        words: &[&[u8]],
        function: &mut FunctionFacts,
    ) -> Result<(), Error> {
        if !function.structured {
            return Err(self.error(
                line,
                "structured ABI name records require a function encoding target",
            ));
        }
        if function.context_records != 0 {
            return Err(self.error(line, "name components cannot follow a local context"));
        }
        if words[0] == b"name-std" {
            if words.len() != 1 {
                return Err(self.error(line, "name-std takes no operands"));
            }
            if !matches!(function.final_component, Component::Missing) || function.scope.is_some() {
                return Err(self.error(line, "name-std must precede the name components"));
            }
            function.scope = Some(self.graph.intern(Node::Std, &[])?);
            return Ok(());
        }
        if words[0] == b"component-abi-tag" {
            if words.len() != 2 {
                return Err(self.error(line, "component-abi-tag takes one tag"));
            }
            if !matches!(function.final_component, Component::Node(_)) {
                return Err(self.error(line, "component ABI tag has no preceding name component"));
            }
            function.component_tags.push(self.graph.string(words[1])?);
            return Ok(());
        }
        self.attach_component(line, function)?;
        if words[0] == b"name-source" {
            if words.len() != 2 && words.len() != 3 {
                return Err(self.error(line, "name-source has invalid operands"));
            }
            function.final_component = self.component(words[1])?;
            return Ok(());
        }
        if words.len() < 7 {
            return Err(self.error(line, "name-template has invalid operands"));
        }
        let args = self.argument_list(&words[6..])?;
        let component = if words[4] == b"-" {
            let name = self.graph.string(words[1])?;
            let template = self.graph.name(function.scope, name, &[])?;
            self.graph.intern(Node::Template(template), &args)?
        } else {
            let kind = Abbreviation::from_word(words[4]).ok_or_else(|| {
                self.word_error(line, "unknown standard substitution '", words[4], "'")
            })?;
            if function
                .scope
                .is_some_and(|scope| self.graph.get(scope) != Node::Std)
            {
                return Err(self.error(
                    line,
                    "a standard substitution must be the first name component",
                ));
            }
            let abbreviation = self.graph.intern(Node::Abbreviation(kind), &[])?;
            let id = if self.flag(line, words[5])? {
                abbreviation
            } else {
                self.graph.intern(Node::Template(abbreviation), &args)?
            };
            function.scope = None;
            id
        };
        function.final_component = Component::Node(component);
        Ok(())
    }
    pub(super) fn finish_function(
        &mut self,
        line: usize,
        mut function: FunctionFacts,
    ) -> Result<NodeId, Error> {
        self.graph.drop_void(&mut function.params);
        if function.terminal.is_none()
            && function.generated_lambda
            && matches!(function.final_component, Component::Missing)
        {
            function.terminal = Some(TerminalFact::Fixed(Terminal::Call));
        }
        let mut scope = function.scope;
        let mut component_tags = function.component_tags;
        let mut component = if let Some(terminal) = function.terminal {
            if let Component::Node(mut component) = function.final_component {
                if self.graph.get(component) == Node::Text {
                    component = self.graph.name(scope, component, &component_tags)?;
                }
                scope = Some(component);
                component_tags.clear();
            }
            match terminal {
                TerminalFact::Node(id) => id,
                TerminalFact::Fixed(kind) => {
                    let operands = function.params.len()
                        + usize::from(scope.is_some_and(|id| self.graph.get(id) != Node::Std));
                    self.graph.intern(
                        Node::Terminal {
                            kind: kind.for_operands(operands),
                            inherited: None,
                        },
                        &[],
                    )?
                }
            }
        } else if let Component::Node(component) = function.final_component {
            component
        } else {
            return Err(self.error(line, "missing ABI function terminal"));
        };
        let mut args = Vec::new();
        if let Node::Template(template) = self.graph.get(component) {
            let Node::Name {
                parent, terminal, ..
            } = self.graph.get(template)
            else {
                return Err(self.error(line, "invalid final structured ABI name component"));
            };
            args.extend_from_slice(self.graph.items(component));
            component_tags = self.graph.items(template).to_vec();
            scope = parent;
            component = terminal;
        } else if matches!(self.graph.get(component), Node::Abbreviation(_)) {
            return Err(self.error(line, "invalid final structured ABI name component"));
        }
        component_tags.extend(function.tags);
        let mut name = self.graph.name(scope, component, &component_tags)?;
        args.extend(function.args);
        if !args.is_empty() {
            name = self.graph.intern(Node::Template(name), &args)?;
        }
        self.graph.intern(
            Node::Function {
                name,
                result: function.result,
                qualifiers: function.qualifiers,
                variadic: function.variadic,
            },
            &function.params,
        )
    }
}
