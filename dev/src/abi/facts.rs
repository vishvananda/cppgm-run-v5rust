//! Explicit PA9 fact-file boundary; compiler producers use the graph directly. See NOTICE.
use super::{
    Error,
    encoder::{CallOffset, Encoder, Target},
    graph::{Graph, Node, NodeId},
    statistics::Statistics,
};
use crate::support::stack::StackBudget;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DefinitionKind {
    Type,
    Argument,
    Expression,
    Context,
    Entity,
}
impl DefinitionKind {
    fn from_word(word: &[u8]) -> Option<Self> {
        Some(match word {
            b"let-type" => Self::Type,
            b"let-arg" => Self::Argument,
            b"let-expr" => Self::Expression,
            b"let-context" => Self::Context,
            b"let-entity" => Self::Entity,
            _ => return None,
        })
    }
}
#[derive(Clone, Copy)]
pub(super) struct Line {
    pub number: u32,
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy)]
enum Resolution {
    Unstarted,
    InProgress,
    Complete(NodeId),
}
struct Definition<'a> {
    id: &'a [u8],
    line: usize,
    kind: DefinitionKind,
    state: Resolution,
}
pub(super) struct CaseReader<'a, 'g> {
    pub graph: &'g mut Graph,
    pub words: Vec<&'a [u8]>,
    pub lines: Vec<Line>,
    definitions: Vec<Definition<'a>>,
    index: HashMap<&'a [u8], usize>,
    budget: StackBudget,
    stack_base: usize,
}
impl<'a, 'g> CaseReader<'a, 'g> {
    fn new(graph: &'g mut Graph, text: &'a [u8], numbers: &[u32], budget: StackBudget) -> Self {
        let mut words = Vec::new();
        let mut lines = Vec::new();
        for (bytes, number) in text.split(|c| *c == b'\n').zip(numbers) {
            let start = words.len();
            words.extend(
                bytes
                    .split(|c| is_space(*c))
                    .filter(|word| !word.is_empty()),
            );
            lines.push(Line {
                number: *number,
                start,
                end: words.len(),
            });
        }
        Self {
            graph,
            words,
            lines,
            definitions: Vec::new(),
            index: HashMap::new(),
            budget,
            stack_base: 0,
        }
    }
    pub fn line_words(&self, line: usize) -> &[&'a [u8]] {
        &self.words[self.lines[line].start..self.lines[line].end]
    }
    pub(super) fn check_type_stack(&self, line: usize) -> Result<(), Error> {
        let marker = 0u8;
        if self
            .budget
            .exceeded(self.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(self.error(line, "ABI definitions nested too deeply"));
        }
        Ok(())
    }
    pub fn error(&self, line: usize, message: impl AsRef<[u8]>) -> Error {
        let mut bytes = format!("ABI fact line {}: ", self.lines[line].number).into_bytes();
        bytes.extend_from_slice(message.as_ref());
        Error::Diagnostic(bytes)
    }
    pub fn word_error(&self, line: usize, prefix: &str, word: &[u8], suffix: &str) -> Error {
        let Error::Diagnostic(bytes) = Error::word(prefix, word, suffix) else {
            return Error::StorageLimit;
        };
        self.error(line, bytes)
    }
    fn index_definitions(&mut self) -> Result<(), Error> {
        // As in C++, collect all incomplete definitions before checking duplicates.
        for line in 0..self.lines.len() {
            let words = self.line_words(line);
            if let Some(kind) = DefinitionKind::from_word(words[0]) {
                if words.len() < 3 {
                    return Err(self.error(line, "incomplete definition"));
                }
                self.definitions.push(Definition {
                    id: words[1],
                    line,
                    kind,
                    state: Resolution::Unstarted,
                });
            }
        }
        for (index, definition) in self.definitions.iter().enumerate() {
            if self.index.insert(definition.id, index).is_some() {
                return Err(self.word_error(
                    definition.line,
                    "duplicate ABI definition id '",
                    definition.id,
                    "'",
                ));
            }
        }
        Ok(())
    }
    fn resolve(&mut self, index: usize) -> Result<NodeId, Error> {
        let definition = &self.definitions[index];
        match definition.state {
            Resolution::Complete(id) => return Ok(id),
            Resolution::InProgress => {
                return Err(self.word_error(
                    definition.line,
                    "recursive ABI definition '",
                    definition.id,
                    "'",
                ));
            }
            Resolution::Unstarted => {}
        }
        let marker = 0u8;
        if self
            .budget
            .exceeded(self.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(self.error(definition.line, "ABI definitions nested too deeply"));
        }
        let line = definition.line;
        let kind = definition.kind;
        self.definitions[index].state = Resolution::InProgress;
        let words = self.line_words(line)[2..].to_vec();
        let id = match kind {
            DefinitionKind::Type => self.type_spec(line, &words)?,
            DefinitionKind::Argument => self.argument_definition(line, &words)?,
            DefinitionKind::Expression => self.expression_definition(line, &words)?,
            DefinitionKind::Context => self.context_definition(line, &words)?,
            DefinitionKind::Entity => self.entity_definition(line, &words)?,
        };
        self.definitions[index].state = Resolution::Complete(id);
        Ok(id)
    }
    pub fn defined_as(&self, word: &[u8], kind: DefinitionKind) -> bool {
        self.index
            .get(word)
            .is_some_and(|i| self.definitions[*i].kind == kind)
    }
    pub fn reference(
        &mut self,
        word: &[u8],
        kind: DefinitionKind,
        what: &str,
    ) -> Result<NodeId, Error> {
        let Some(index) = self
            .index
            .get(word)
            .copied()
            .filter(|i| self.definitions[*i].kind == kind)
        else {
            return Err(Error::word(&format!("unknown ABI {what} '"), word, "'"));
        };
        self.resolve(index)
    }
    pub fn index_value(&self, line: usize, word: &[u8]) -> Result<u32, Error> {
        if word.is_empty() || word.len() > 9 || !word.iter().all(u8::is_ascii_digit) {
            return Err(self.word_error(line, "invalid ABI index '", word, "'"));
        }
        Ok(word
            .iter()
            .fold(0, |value, digit| value * 10 + u32::from(digit - b'0')))
    }
    pub fn integer_value(&self, line: usize, word: &[u8]) -> Result<i64, Error> {
        let negative = word.starts_with(b"-");
        let digits = if negative { &word[1..] } else { word };
        if digits.is_empty() {
            return Err(self.word_error(line, "invalid ABI integer '", word, "'"));
        }
        let mut magnitude = 0u64;
        for digit in digits {
            if !digit.is_ascii_digit() {
                return Err(self.word_error(line, "invalid ABI integer '", word, "'"));
            }
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|value| value.checked_add(u64::from(digit - b'0')))
                .ok_or_else(|| self.word_error(line, "ABI integer out of range '", word, "'"))?;
        }
        if negative && magnitude > (1 << 63) {
            return Err(self.word_error(line, "ABI integer out of range '", word, "'"));
        }
        let bits = if negative {
            0u64.wrapping_sub(magnitude)
        } else {
            magnitude
        };
        Ok(i64::from_ne_bytes(bits.to_ne_bytes()))
    }
    pub fn flag(&self, line: usize, word: &[u8]) -> Result<bool, Error> {
        match word {
            b"yes" | b"true" | b"1" => Ok(true),
            b"no" | b"false" | b"0" => Ok(false),
            _ => Err(self.word_error(line, "expected boolean ABI word, got '", word, "'")),
        }
    }
    pub fn qualified_name(&mut self, line: usize, path: &[u8]) -> Result<NodeId, Error> {
        let path = path.strip_prefix(b"::").unwrap_or(path);
        if path.is_empty() {
            return Err(self.error(line, "empty ABI qualified name"));
        }
        let mut parent = None;
        let mut start = 0;
        loop {
            let end = path[start..]
                .windows(2)
                .position(|w| w == b"::")
                .map_or(path.len(), |n| start + n);
            if start == end {
                return Err(self.word_error(line, "empty component in ABI name '", path, "'"));
            }
            let part = &path[start..end];
            if part.contains(&b':') {
                return Err(self.word_error(line, "stray ':' in ABI name '", path, "'"));
            }
            let id = if start == 0 && end < path.len() && part == b"std" {
                self.graph.intern(Node::Std, &[])?
            } else {
                let terminal = self.graph.string(part)?;
                self.graph.name(parent, terminal, &[])?
            };
            if end == path.len() {
                return Ok(id);
            }
            parent = Some(id);
            start = end + 2;
        }
    }
    fn read_target(&mut self) -> Result<Target, Error> {
        let marker = 0u8;
        self.stack_base = std::ptr::from_ref(&marker).addr();
        self.index_definitions()?;
        for index in 0..self.definitions.len() {
            self.resolve(index)?;
        }
        let mut target_line = None;
        for line in 0..self.lines.len() {
            if is_target(self.line_words(line)[0]) {
                if target_line.is_some() {
                    return Err(self.error(line, "ABI case has more than one target"));
                }
                target_line = Some(line);
            }
        }
        let line = target_line.ok_or_else(|| Error::diagnostic("ABI case has no target"))?;
        let words = self.line_words(line).to_vec();
        let function_start = match words[0] {
            b"function" => Some(1),
            b"thunk" | b"virtual-base-thunk" => Some(self.thunk_function_start(line, &words)?),
            _ => None,
        };
        let target = if let Some(start) = function_start {
            let mut function = self.function_target(line, &words[start..])?;
            for record in 0..self.lines.len() {
                if record != line && DefinitionKind::from_word(self.line_words(record)[0]).is_none()
                {
                    self.function_record(record, &mut function)?;
                }
            }
            let id = self.finish_function(line, function)?;
            if start == 1 {
                Target::Entity(id)
            } else {
                self.thunk_target(line, &words, id)?
            }
        } else {
            let target = self.nonfunction_target(line, &words)?;
            for record in 0..self.lines.len() {
                if record != line && DefinitionKind::from_word(self.line_words(record)[0]).is_none()
                {
                    return Err(self.error(
                        record,
                        if words[0] == b"c-function" {
                            "C-linkage function cannot have ABI function records"
                        } else {
                            "function records attached to a non-function ABI target"
                        },
                    ));
                }
            }
            target
        };
        Ok(target)
    }
    fn nonfunction_target(&mut self, line: usize, words: &[&[u8]]) -> Result<Target, Error> {
        match words[0] {
            b"type" | b"typeinfo" | b"typeinfo-name" | b"vtable" | b"vtt" => {
                let id = self.type_spec(line, &words[1..])?;
                Ok(match words[0] {
                    b"type" => Target::Type(id),
                    b"typeinfo" => Target::Typeinfo(id),
                    b"typeinfo-name" => Target::TypeinfoName(id),
                    b"vtable" => Target::Vtable(id),
                    _ => Target::Vtt(id),
                })
            }
            b"variable" | b"tls-wrapper" => {
                let wrapper = words[0] == b"tls-wrapper";
                if words.len() != (if wrapper { 3 } else { 2 })
                    || (wrapper && words[1] != b"variable")
                {
                    return Err(self.error(
                        line,
                        if wrapper {
                            "TLS wrapper must name a variable"
                        } else {
                            "variable target takes one name"
                        },
                    ));
                }
                let id = self.qualified_name(line, words[words.len() - 1])?;
                if wrapper {
                    return Ok(Target::TlsWrapper(id));
                }
                if let Node::Name {
                    parent: None,
                    terminal,
                    internal: false,
                    data_member: false,
                    occurrence: 0,
                } = self.graph.get(id)
                {
                    return Ok(Target::CName(terminal));
                }
                Ok(Target::Entity(id))
            }
            b"c-function" => {
                if words.len() < 2 {
                    return Err(self.error(line, "empty C-linkage ABI function name"));
                }
                let id = self.qualified_name(line, words[1])?;
                for word in &words[2..] {
                    self.compact_type(line, word)?;
                }
                let Node::Name { terminal, .. } = self.graph.get(id) else {
                    return Err(Error::StorageLimit);
                };
                Ok(Target::CName(terminal))
            }
            b"construction-vtable" => {
                if words.len() != 4 {
                    return Err(
                        self.error(line, "construction vtable needs derived, offset, and base")
                    );
                }
                let derived = self.compact_type(line, words[1])?;
                let offset = self.integer_value(line, words[2])?;
                let base = self.compact_type(line, words[3])?;
                Ok(Target::ConstructionVtable {
                    derived,
                    offset,
                    base,
                })
            }
            _ => Err(Error::NotImplemented("ABI target")),
        }
    }
    fn thunk_function_start(&self, line: usize, words: &[&[u8]]) -> Result<usize, Error> {
        if words.len() < 3 {
            return Err(self.error(line, "thunk is missing function target"));
        }
        self.integer_value(line, words[1])?;
        let next = if words[0] == b"virtual-base-thunk" {
            2
        } else if words[2] == b"virtual-result" {
            if words.len() < 6 {
                return Err(self.error(line, "virtual result thunk has invalid offsets"));
            }
            self.integer_value(line, words[3])?;
            self.integer_value(line, words[4])?;
            5
        } else if words[2] != b"function" {
            self.integer_value(line, words[2])?;
            3
        } else {
            2
        };
        if words.get(next) != Some(&b"function".as_slice()) {
            return Err(self.error(line, "thunk target has invalid operands"));
        }
        Ok(next + 1)
    }
    fn thunk_target(
        &self,
        line: usize,
        words: &[&[u8]],
        function: NodeId,
    ) -> Result<Target, Error> {
        let adjustment = self.integer_value(line, words[1])?;
        let this = if words[0] == b"virtual-base-thunk" {
            CallOffset {
                offset: 0,
                vcall: Some(adjustment),
            }
        } else {
            CallOffset {
                offset: adjustment,
                vcall: None,
            }
        };
        let result = if words[0] == b"virtual-base-thunk" || words[2] == b"function" {
            None
        } else if words[2] == b"virtual-result" {
            Some(CallOffset {
                offset: self.integer_value(line, words[3])?,
                vcall: Some(self.integer_value(line, words[4])?),
            })
        } else {
            Some(CallOffset {
                offset: self.integer_value(line, words[2])?,
                vcall: None,
            })
        };
        Ok(Target::Thunk {
            function,
            this,
            result,
        })
    }
}
fn is_target(word: &[u8]) -> bool {
    matches!(
        word,
        b"type"
            | b"function"
            | b"variable"
            | b"c-function"
            | b"typeinfo"
            | b"typeinfo-name"
            | b"vtable"
            | b"vtt"
            | b"construction-vtable"
            | b"tls-wrapper"
            | b"thunk"
            | b"virtual-base-thunk"
    )
}
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | 11 | 12)
}

/// Read and encode file-local cases in order, retaining only the active case.
///
/// # Errors
/// Returns exact fact diagnostics, I/O failures, or an explicit unported group.
pub fn mangle_files(
    paths: &[PathBuf],
    output: &mut impl Write,
    budget: StackBudget,
) -> Result<Statistics, Error> {
    let mut stats = Statistics::default();
    let mut graph = Graph::default();
    let mut pending_output = Vec::new();
    for path in paths {
        let input = File::open(path).map_err(|_| {
            Error::word(
                "unable to open ABI fact file '",
                path.as_os_str().as_encoded_bytes(),
                "'",
            )
        })?;
        let mut input = BufReader::with_capacity(64 << 10, input);
        let mut line = Vec::new();
        let mut text = Vec::new();
        let mut numbers = Vec::new();
        let mut open = false;
        let mut cases = 0;
        let mut number = 0u32;
        loop {
            line.clear();
            if input.read_until(b'\n', &mut line).map_err(|_| {
                Error::word(
                    "unable to read ABI fact file '",
                    path.as_os_str().as_encoded_bytes(),
                    "'",
                )
            })? == 0
            {
                break;
            }
            stats.source_bytes += line.len();
            number = number.checked_add(1).ok_or(Error::StorageLimit)?;
            let bytes = line.strip_suffix(b"\n").unwrap_or(&line);
            let start = bytes
                .iter()
                .position(|c| !is_space(*c))
                .unwrap_or(bytes.len());
            let bytes = &bytes[start..];
            if bytes.is_empty() || bytes[0] == b'#' {
                continue;
            }
            let first_end = bytes
                .iter()
                .position(|c| is_space(*c))
                .unwrap_or(bytes.len());
            if &bytes[..first_end] == b"case" {
                if bytes
                    .split(|c| is_space(*c))
                    .filter(|w| !w.is_empty())
                    .count()
                    != 2
                {
                    return Err(Error::diagnostic(format!(
                        "ABI fact line {number}: case takes one label"
                    )));
                }
                if open {
                    finish_case(
                        &mut graph,
                        &text,
                        &numbers,
                        &mut pending_output,
                        output,
                        budget,
                        &mut stats,
                    )?;
                    cases += 1;
                }
                text.clear();
                numbers.clear();
                open = true;
            } else {
                text.extend_from_slice(bytes);
                text.push(b'\n');
                numbers.push(number);
                open = true;
            }
        }
        if open {
            finish_case(
                &mut graph,
                &text,
                &numbers,
                &mut pending_output,
                output,
                budget,
                &mut stats,
            )?;
            cases += 1;
        }
        stats.source_files += 1;
        if cases == 0 {
            return Err(Error::diagnostic("ABI fact file contains no cases"));
        }
    }
    output
        .write_all(&pending_output)
        .map_err(|_| Error::diagnostic("unable to write mangled ABI output"))?;
    Ok(stats)
}
fn finish_case(
    graph: &mut Graph,
    text: &[u8],
    numbers: &[u32],
    pending: &mut Vec<u8>,
    output: &mut impl Write,
    budget: StackBudget,
    stats: &mut Statistics,
) -> Result<(), Error> {
    let mut reader = CaseReader::new(graph, text, numbers, budget);
    let target = reader.read_target()?;
    let definitions = reader.definitions.len();
    Encoder::encode_observed(graph, target, pending, budget, Some(stats))?;
    stats.cases += 1;
    stats.records += numbers.len();
    stats.definitions += definitions;
    stats.peak_graph_nodes = stats.peak_graph_nodes.max(graph.len());
    stats.peak_graph_bytes = stats.peak_graph_bytes.max(graph.bytes_reserved());
    pending.push(b'\n');
    graph.reset();
    if pending.len() >= 64 << 10 {
        output
            .write_all(pending)
            .map_err(|_| Error::diagnostic("unable to write mangled ABI output"))?;
        pending.clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_facts_resolve_once_and_definition_cycles_reject() {
        let mut graph = Graph::default();
        let text = b"let-type A ptr B\nlet-type B int\nfunction f A A\n";
        let mut reader = CaseReader::new(&mut graph, text, &[1, 2, 3], StackBudget::default());
        let target = reader.read_target().unwrap();
        let before = reader.graph.len();
        let first = reader
            .reference(b"A", DefinitionKind::Type, "type")
            .unwrap();
        let second = reader
            .reference(b"A", DefinitionKind::Type, "type")
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(reader.graph.len(), before);
        let mut output = Vec::new();
        Encoder::encode(&graph, target, &mut output, StackBudget::default()).unwrap();
        assert_eq!(output, b"_Z1fPiS_");
        graph.reset();
        let text = b"let-type A ptr B\nlet-type B A\ntype A\n";
        let mut reader = CaseReader::new(&mut graph, text, &[4, 5, 6], StackBudget::default());
        assert!(
            matches!(reader.read_target(), Err(Error::Diagnostic(bytes)) if bytes == b"ABI fact line 4: recursive ABI definition 'A'")
        );
    }
}
