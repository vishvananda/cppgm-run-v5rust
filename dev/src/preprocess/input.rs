// Translated from preprocess/source/source_stack.cpp (source frames). See NOTICE.
//! Demand-driven text/directive boundary, with stable source views and interned names.
use super::error::PreprocessError;
use super::files::{FileId, FileServices, SourceFiles};
use super::macros::token::{LEADING_SPACE, START_OF_LINE, Spelling, Token};
use crate::lex::identifiers::{Identifiers, NameId};
use crate::lex::pp_lexer::{PpLexer, PpLexerOptions, PpTokenKind, Punctuator};
use crate::support::source::{SourceBuffer, SourceImage, SourceOffset};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Boundary {
    Text,
    Directive,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Text,
    Line,
    LineEnd,
}

#[derive(Clone, Copy)]
struct InclusionId(u32);
struct Inclusion<'a> {
    base: u32,
    parent: Option<InclusionId>,
    included_at: SourceOffset,
    locations: super::locations::Locations<'a>,
}
#[derive(Clone, Copy)]
enum GuardState {
    Start,
    Open { name: NameId, depth: usize },
    Closed(NameId),
    None,
}
struct SourceFrame<'a> {
    source: SourceImage<'a>,
    lexer: PpLexer<'a>,
    at_line_start: bool,
    inclusion: InclusionId,
    file: FileId,
    conditional_depth: usize,
    guard: GuardState,
    open_conditional: Option<SourceOffset>,
    search_index: Option<usize>,
}
pub(crate) struct SourceInput<'a> {
    frame: SourceFrame<'a>,
    suspended: Vec<SourceFrame<'a>>,
    inclusions: Vec<Inclusion<'a>>,
    files: SourceFiles<'a>,
    next_base: u32,
    boundary: Boundary,
    location: SourceOffset,
    mode: InputMode,
    peeked: Option<Token<'a>>,
    pub(crate) allow_va_args: bool,
    predefined: Option<FileId>,
    command_line: std::ops::Range<u32>,
    host_predefines: std::ops::Range<u32>,
    tokens_read: u64,
    includes: u64,
    guarded_skips: u64,
    binary_exponents: bool,
    pub(crate) hosted: bool,
    include_paths: Vec<Vec<u8>>,
    pub(crate) modes: super::options::FeatureModes,
}
struct IncludeMatch {
    file: FileId,
    path: Vec<u8>,
    search_index: Option<usize>,
}
impl<'a> SourceInput<'a> {
    pub(crate) fn set_include_paths(&mut self, options: &super::options::DriverOptions) {
        self.hosted = options.hosted;
        self.modes = options.modes;
        self.include_paths.clone_from(&options.include_paths);
        self.include_paths
            .extend_from_slice(&options.system_include_paths);
    }

    pub(crate) fn set_binary_exponents(&mut self, enabled: bool) {
        self.binary_exponents = enabled;
        self.frame.lexer.set_binary_exponents(enabled);
        for frame in &mut self.suspended {
            frame.lexer.set_binary_exponents(enabled);
        }
    }
    #[cfg(test)]
    pub(crate) fn with_name(
        source: &'a SourceBuffer,
        name: &[u8],
    ) -> Result<Self, PreprocessError> {
        Self::with_file_services(
            source,
            name,
            FileServices {
                identity: |_| None,
                read: |_| None,
            },
        )
    }
    pub(crate) fn with_file_services(
        source: &'a SourceBuffer,
        name: &[u8],
        services: FileServices,
    ) -> Result<Self, PreprocessError> {
        let files = SourceFiles::new(source, name, services);
        Ok(Self {
            frame: SourceFrame {
                source: SourceImage::Borrowed(source),
                lexer: PpLexer::with_options(
                    source,
                    PpLexerOptions {
                        header_names_on_request: true,
                        ..PpLexerOptions::default()
                    },
                )?,
                at_line_start: true,
                inclusion: InclusionId(0),
                file: SourceFiles::primary(),
                conditional_depth: 0,
                guard: GuardState::Start,
                open_conditional: None,
                search_index: None,
            },
            suspended: Vec::new(),
            inclusions: vec![Inclusion {
                base: 0,
                parent: None,
                included_at: source.begin(),
                locations: super::locations::Locations::new(source, name),
            }],
            next_base: Self::next_base(0, source)?,
            files,
            boundary: Boundary::Text,
            location: source.begin(),
            mode: InputMode::Text,
            peeked: None,
            allow_va_args: false,
            predefined: None,
            command_line: 0..0,
            host_predefines: 0..0,
            tokens_read: 0,
            includes: 0,
            guarded_skips: 0,
            binary_exponents: false,
            hosted: false,
            include_paths: Vec::new(),
            modes: super::options::FeatureModes::default(),
        })
    }
    fn next_base(base: u32, source: &SourceBuffer) -> Result<u32, PreprocessError> {
        let size = u32::try_from(source.bytes().len()).map_err(|_| {
            PreprocessError::Limit("the translation unit exceeds the source location space")
        })?;
        base.checked_add(size)
            .and_then(|end| end.checked_add(1))
            .ok_or_else(|| {
                PreprocessError::at(
                    SourceOffset(0),
                    "the translation unit exceeds the source location space",
                )
            })
    }
    fn inclusion(&self) -> &Inclusion<'a> {
        &self.inclusions[self.frame.inclusion.0 as usize]
    }
    fn locations(&self) -> &super::locations::Locations<'a> {
        &self.inclusion().locations
    }
    fn locations_mut(&mut self) -> &mut super::locations::Locations<'a> {
        &mut self.inclusions[self.frame.inclusion.0 as usize].locations
    }
    fn stabilize(
        &mut self,
        kind: PpTokenKind,
        flags: u8,
        names: &mut Identifiers,
    ) -> Result<Token<'a>, PreprocessError> {
        self.tokens_read = self.tokens_read.wrapping_add(1);
        let pp = self.frame.lexer.token(kind);
        let mut token = Token::end(pp.begin);
        token.kind = kind;
        token.end = pp.end;
        token.punctuator = pp.punctuator;
        token.flags = flags;
        if kind == PpTokenKind::Identifier {
            let name = names.intern(pp.spelling).map_err(PreprocessError::Limit)?;
            if pp.spelling == b"__VA_ARGS__" && !self.allow_va_args {
                return Err(PreprocessError::at(
                    pp.begin,
                    "__VA_ARGS__ outside a variadic macro's replacement list",
                ));
            }
            token.name = Some(name);
            token.spelling = Spelling::Shared(names.spelling(name));
        } else {
            let base = self.inclusion().base;
            let begin = SourceOffset(pp.begin.0 - base);
            let end = SourceOffset(pp.end.0 - base);
            token.spelling = match &self.frame.source {
                SourceImage::Borrowed(source)
                    if source.bytes().get(begin.index()..end.index()) == Some(pp.spelling) =>
                {
                    Spelling::Source(&source.bytes()[begin.index()..end.index()])
                }
                SourceImage::Shared(source)
                    if source.bytes().get(begin.index()..end.index()) == Some(pp.spelling) =>
                {
                    Spelling::Image {
                        source: source.clone(),
                        begin,
                        end,
                    }
                }
                _ => Spelling::Shared(pp.spelling.into()),
            };
        }
        Ok(token)
    }
    pub(crate) fn next(&mut self, names: &mut Identifiers) -> Result<Token<'a>, PreprocessError> {
        if let Some(token) = self.peeked.take() {
            return Ok(token);
        }
        if self.mode != InputMode::Text {
            if self.mode == InputMode::LineEnd {
                return Ok(Token::end(self.location));
            }
            let token = self.next_in_line(names)?;
            if token.kind == PpTokenKind::NewLine {
                self.mode = InputMode::LineEnd;
                self.location = token.begin;
                return Ok(Token::end(token.begin));
            }
            return Ok(token);
        }
        if self.boundary != Boundary::Text {
            return Ok(Token::end(self.location));
        }
        let mut flags = if self.frame.at_line_start {
            LEADING_SPACE | START_OF_LINE
        } else {
            0
        };
        loop {
            let kind = self.frame.lexer.next_token()?.kind;
            match kind {
                PpTokenKind::WhitespaceSequence => {
                    flags |= LEADING_SPACE;
                    continue;
                }
                PpTokenKind::NewLine => {
                    flags |= LEADING_SPACE | START_OF_LINE;
                    self.frame.at_line_start = true;
                    continue;
                }
                PpTokenKind::EndOfFile => {
                    if !self.suspended.is_empty() {
                        self.leave_file()?;
                        flags |= LEADING_SPACE | START_OF_LINE;
                        continue;
                    }
                    self.boundary = Boundary::End;
                    self.location = self.frame.lexer.token(kind).begin;
                    return Ok(Token::end(self.location));
                }
                _ => {}
            }
            self.frame.at_line_start = false;
            if flags & START_OF_LINE != 0
                && self.frame.lexer.token(kind).punctuator == Some(Punctuator::Hash)
            {
                self.boundary = Boundary::Directive;
                self.location = self.frame.lexer.token(kind).begin;
                return Ok(Token::end(self.location));
            }
            if !matches!(self.frame.guard, GuardState::Open { .. }) {
                self.frame.guard = GuardState::None;
            }
            return self.stabilize(kind, flags, names);
        }
    }
    pub(crate) fn peek(&mut self, names: &mut Identifiers) -> Result<&Token<'a>, PreprocessError> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next(names)?);
        }
        self.peeked
            .as_ref()
            .ok_or(PreprocessError::Invariant("missing source lookahead"))
    }
    pub(crate) fn next_in_line(
        &mut self,
        names: &mut Identifiers,
    ) -> Result<Token<'a>, PreprocessError> {
        let mut flags = 0;
        loop {
            let kind = self.frame.lexer.next_token()?.kind;
            if kind == PpTokenKind::WhitespaceSequence {
                flags |= LEADING_SPACE;
                continue;
            }
            if matches!(kind, PpTokenKind::NewLine | PpTokenKind::EndOfFile) {
                self.frame.at_line_start = true;
                let mut token = Token::end(self.frame.lexer.token(kind).begin);
                token.kind = PpTokenKind::NewLine;
                return Ok(token);
            }
            return self.stabilize(kind, flags, names);
        }
    }
    pub(crate) fn begin_line_input(&mut self) {
        self.mode = InputMode::Line;
        self.peeked = None;
    }
    pub(crate) fn begin_line_with(&mut self, first: Token<'a>) {
        self.begin_line_input();
        self.peeked = Some(first);
    }
    pub(crate) fn end_line_input(&mut self) -> Result<(), PreprocessError> {
        if self.mode != InputMode::LineEnd {
            self.skip_line()?;
        }
        self.mode = InputMode::Text;
        self.peeked = None;
        Ok(())
    }
    pub(crate) fn request_header_name(&mut self) {
        self.frame.lexer.request_header_name();
    }
    pub(crate) fn skip_line(&mut self) -> Result<(), PreprocessError> {
        if self.mode == InputMode::LineEnd {
            return Ok(());
        }
        self.peeked = None;
        loop {
            let kind = self.frame.lexer.next_token()?.kind;
            if matches!(kind, PpTokenKind::NewLine | PpTokenKind::EndOfFile) {
                self.frame.at_line_start = true;
                if self.mode != InputMode::Text {
                    self.mode = InputMode::LineEnd;
                }
                return Ok(());
            }
        }
    }
    // Inactive text is lexed for boundaries only: it is never stabilized or interned.
    pub(crate) fn skip_to_directive(&mut self) -> Result<bool, PreprocessError> {
        let mut line_start = self.frame.at_line_start;
        self.peeked = None;
        loop {
            let pp = self.frame.lexer.next_token()?;
            match pp.kind {
                PpTokenKind::WhitespaceSequence => continue,
                PpTokenKind::NewLine => {
                    line_start = true;
                    continue;
                }
                PpTokenKind::EndOfFile => {
                    self.frame.at_line_start = true;
                    return Ok(false);
                }
                _ => {}
            }
            if line_start && pp.punctuator == Some(Punctuator::Hash) {
                self.frame.at_line_start = false;
                self.location = pp.begin;
                return Ok(true);
            }
            line_start = false;
        }
    }
    pub(crate) fn at_directive(&self) -> bool {
        self.boundary == Boundary::Directive
    }
    pub(crate) fn end_directive(&mut self) {
        self.boundary = Boundary::Text;
        self.peeked = None;
    }
    pub(crate) fn presumed_line(&self, location: SourceOffset) -> Result<i64, PreprocessError> {
        // Builtins use the inclusion owning their invocation, with its current line delta.
        let inclusion = self.inclusion_at(location);
        inclusion
            .locations
            .presumed_line(SourceOffset(location.0 - inclusion.base))
    }
    pub(crate) fn file_literal(&mut self, location: SourceOffset) -> std::rc::Rc<[u8]> {
        let index = self
            .inclusions
            .partition_point(|inclusion| inclusion.base <= location.0)
            .saturating_sub(1);
        self.inclusions[index].locations.file_literal()
    }
    pub(crate) fn presumed_name(&self) -> &[u8] {
        self.locations().presumed_name()
    }
    pub(crate) fn set_line(
        &mut self,
        number: u32,
        name: Option<Vec<u8>>,
    ) -> Result<(), PreprocessError> {
        let location = SourceOffset(self.location.0 - self.inclusion().base);
        self.locations_mut().set_line(location, number, name)
    }
    fn inclusion_at(&self, location: SourceOffset) -> &Inclusion<'a> {
        &self.inclusions[self
            .inclusions
            .partition_point(|inclusion| inclusion.base <= location.0)
            .saturating_sub(1)]
    }
    pub(crate) fn describe(
        &self,
        error: &crate::support::source::SourceError,
    ) -> Result<Vec<u8>, PreprocessError> {
        let location = SourceOffset(
            self.inclusion().base
                + error
                    .offset
                    .0
                    .min(u32::try_from(self.frame.source.bytes().len()).map_err(|_| {
                        PreprocessError::Invariant("source exceeds location space")
                    })?),
        );
        self.describe_at(location, error.message)
    }
    pub(crate) fn describe_at(
        &self,
        location: SourceOffset,
        message: &'static str,
    ) -> Result<Vec<u8>, PreprocessError> {
        let mut inclusion = self.inclusion_at(location);
        let mut text = inclusion
            .locations
            .describe(&crate::support::source::SourceError::new(
                SourceOffset(location.0 - inclusion.base),
                message,
            ))?;
        while let Some(parent) = inclusion.parent {
            let at = inclusion.included_at;
            inclusion = &self.inclusions[parent.0 as usize];
            let (name, line) = inclusion
                .locations
                .line_and_name(SourceOffset(at.0 - inclusion.base))?;
            text.extend_from_slice(b"\nnote: included from ");
            text.extend_from_slice(&name);
            text.extend_from_slice(format!(":{line}").as_bytes());
        }
        Ok(text)
    }
    #[cfg(test)]
    pub(crate) fn inclusion_count(&self) -> usize {
        self.inclusions.len()
    }
    pub(crate) fn conditional_depth(&self) -> usize {
        self.frame.conditional_depth
    }
    pub(crate) fn track_guard(&mut self, directive: &[u8], depth: usize) {
        self.frame.guard = match self.frame.guard {
            GuardState::Start if directive != b"ifndef" => GuardState::None,
            GuardState::Open {
                name,
                depth: guard_depth,
            } if depth == guard_depth + 1 => match directive {
                b"endif" => GuardState::Closed(name),
                b"elif" | b"else" => GuardState::None,
                _ => self.frame.guard,
            },
            GuardState::Closed(_) => GuardState::None,
            _ => self.frame.guard,
        };
    }
    pub(crate) fn open_guard(&mut self, name: NameId, depth: usize) {
        if matches!(self.frame.guard, GuardState::Start) {
            self.frame.guard = GuardState::Open { name, depth };
        }
    }
    pub(crate) fn set_open_conditional(&mut self, location: Option<SourceOffset>) {
        self.frame.open_conditional = location;
    }
    fn leave_file(&mut self) -> Result<(), PreprocessError> {
        if let Some(location) = self.frame.open_conditional {
            return Err(PreprocessError::at(
                location,
                "unterminated conditional directive",
            ));
        }
        if let GuardState::Closed(name) = self.frame.guard {
            self.files.set_guard(self.frame.file, name);
        }
        self.frame = self
            .suspended
            .pop()
            .ok_or(PreprocessError::Invariant("missing parent source frame"))?;
        Ok(())
    }
    fn search_directories(
        &mut self,
        name: &[u8],
        start: usize,
    ) -> Result<Option<IncludeMatch>, PreprocessError> {
        for (index, directory) in self.include_paths.iter().enumerate().skip(start) {
            let mut path = directory.clone();
            if !path.is_empty() && !path.ends_with(b"/") {
                path.push(b'/');
            }
            path.extend_from_slice(name);
            if let Some(file) = self.files.open(&path)? {
                return Ok(Some(IncludeMatch {
                    file,
                    path,
                    search_index: Some(index),
                }));
            }
        }
        Ok(None)
    }
    fn find_include(
        &mut self,
        name: &[u8],
        angle: bool,
        next: bool,
    ) -> Result<Option<IncludeMatch>, PreprocessError> {
        if next {
            return self
                .search_directories(name, self.frame.search_index.map_or(0, |index| index + 1));
        }
        if angle && let Some(found) = self.search_directories(name, 0)? {
            return Ok(Some(found));
        }
        let current = self.presumed_name();
        let mut path = current
            .iter()
            .rposition(|&b| b == b'/')
            .map_or_else(Vec::new, |slash| current[..=slash].to_vec());
        path.extend_from_slice(name);
        if let Some(file) = self.files.open(&path)? {
            return Ok(Some(IncludeMatch {
                file,
                path,
                search_index: None,
            }));
        }
        if !angle && let Some(found) = self.search_directories(name, 0)? {
            return Ok(Some(found));
        }
        Ok(self.files.open(name)?.map(|file| IncludeMatch {
            file,
            path: name.to_vec(),
            search_index: None,
        }))
    }
    pub(crate) fn can_include(
        &mut self,
        name: &[u8],
        angle: bool,
        next: bool,
    ) -> Result<bool, PreprocessError> {
        Ok(self.find_include(name, angle, next)?.is_some())
    }
    pub(crate) fn include(
        &mut self,
        directive: &Token<'a>,
        name: &[u8],
        angle: bool,
        depth: usize,
        definitions: &super::macros::definition::Definitions<'a>,
        pragmas: &super::pragmas::Pragmas,
    ) -> Result<(), PreprocessError> {
        let IncludeMatch {
            file,
            path,
            search_index,
        } = self
            .find_include(name, angle, directive.spelling.bytes() == b"include_next")?
            .ok_or_else(|| PreprocessError::at(directive.begin, "include file not found"))?;
        if self
            .files
            .identity(file)
            .is_some_and(|id| pragmas.is_once(id))
        {
            return Ok(());
        }
        if self
            .files
            .guard(file)
            .is_some_and(|name| definitions.is_defined(name))
        {
            self.guarded_skips = self.guarded_skips.wrapping_add(1);
            return Ok(());
        }
        if self.suspended.len() + 1 > 200 {
            return Err(PreprocessError::at(
                directive.begin,
                "#include nested too deeply",
            ));
        }
        self.includes = self.includes.wrapping_add(1);
        self.enter(file, &path, depth, Some(directive.begin))?;
        self.frame.search_index = search_index;
        Ok(())
    }
    pub(crate) fn enter_command_line(&mut self, text: &[u8]) -> Result<(), PreprocessError> {
        if !text.is_empty() {
            let file = self.files.add_text(text.to_vec())?;
            self.enter(file, b"<command-line>", 0, None)?;
            self.command_line = self.inclusion().base..self.next_base;
        }
        Ok(())
    }
    pub(crate) fn enter_predefined(
        &mut self,
        text: Vec<u8>,
        host_predefines_begin: usize,
    ) -> Result<(), PreprocessError> {
        let file = self.files.add_text(text)?;
        self.predefined = Some(file);
        self.enter(file, b"<predefined>", 0, None)?;
        let host_begin = self
            .inclusion()
            .base
            .checked_add(
                u32::try_from(host_predefines_begin)
                    .map_err(|_| PreprocessError::Limit("host predefined source space"))?,
            )
            .ok_or(PreprocessError::Limit("host predefined source space"))?;
        self.host_predefines = host_begin..self.next_base;
        Ok(())
    }
    pub(crate) fn enter_builtin_declarations(
        &mut self,
        text: &[u8],
    ) -> Result<(), PreprocessError> {
        let file = self.files.add_text(text.to_vec())?;
        let predefined = self.in_predefined();
        self.enter(file, b"<builtins>", 0, None)?;
        // Predefined macros precede the grammar prefix, command-line input and
        // primary source. Installing the prefix must not demand any tokens.
        if predefined {
            let frame = self
                .suspended
                .last_mut()
                .ok_or(PreprocessError::Invariant("builtin source order"))?;
            std::mem::swap(&mut self.frame, frame);
        }
        Ok(())
    }
    pub(crate) fn command_line_definition(&self, at: SourceOffset) -> bool {
        self.command_line.contains(&at.0) || self.host_predefines.contains(&at.0)
    }

    pub(crate) fn in_predefined(&self) -> bool {
        self.predefined == Some(self.frame.file)
    }
    pub(crate) fn counters(&self) -> super::counters::PreprocessorCounters {
        let (files, source_bytes) = self.files.counts();
        super::counters::PreprocessorCounters {
            source_tokens: self.tokens_read,
            includes: self.includes,
            guarded_skips: self.guarded_skips,
            files,
            source_bytes,
            ..super::counters::PreprocessorCounters::default()
        }
    }
    fn enter(
        &mut self,
        file: FileId,
        path: &[u8],
        depth: usize,
        included_at: Option<SourceOffset>,
    ) -> Result<(), PreprocessError> {
        let image = self.files.image(file);
        let base = self.next_base;
        let next_base = Self::next_base(base, &image)?;
        let inclusion = InclusionId(
            u32::try_from(self.inclusions.len())
                .map_err(|_| PreprocessError::Limit("too many source inclusions"))?,
        );
        let lexer = PpLexer::with_image(
            image.clone(),
            PpLexerOptions {
                location_base: base,
                header_names_on_request: true,
                binary_exponents: self.binary_exponents,
                ..PpLexerOptions::default()
            },
        )?;
        self.inclusions.push(Inclusion {
            base,
            parent: included_at.map(|_| self.frame.inclusion),
            included_at: included_at.unwrap_or(SourceOffset(0)),
            locations: super::locations::Locations::with_image(image.clone(), path),
        });
        let frame = SourceFrame {
            source: image,
            lexer,
            at_line_start: true,
            inclusion,
            file,
            conditional_depth: depth,
            guard: GuardState::Start,
            open_conditional: None,
            search_index: None,
        };
        self.suspended
            .push(std::mem::replace(&mut self.frame, frame));
        self.next_base = next_base;
        Ok(())
    }
}

impl super::locations::SourcePositions for SourceInput<'_> {
    fn source_file(&self, location: SourceOffset) -> std::rc::Rc<[u8]> {
        let inclusion = self.inclusion_at(location);
        inclusion
            .locations
            .name_at(SourceOffset(location.0 - inclusion.base))
    }
    fn source_line(&self, location: SourceOffset) -> Result<i64, PreprocessError> {
        let inclusion = self.inclusion_at(location);
        inclusion
            .locations
            .line_at(SourceOffset(location.0 - inclusion.base))
    }
}
