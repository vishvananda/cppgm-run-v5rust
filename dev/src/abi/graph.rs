//! Canonical ABI identities in flat case/unit-owned stores. See NOTICE.
use super::{
    Error,
    vocabulary::{Builtin, Operation, Terminal},
};
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    num::NonZeroU32,
    ops::Range,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(NonZeroU32);
impl NodeId {
    pub(crate) fn index(self) -> usize {
        self.0.get() as usize
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Qualifiers(u8);
impl Qualifiers {
    pub const CONST: Self = Self(1);
    pub const VOLATILE: Self = Self(2);
    pub const RESTRICT: Self = Self(4);
    pub const LVALUE: Self = Self(8);
    pub const RVALUE: Self = Self(16);
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[must_use]
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Abbreviation {
    Allocator,
    BasicString,
    String,
    Istream,
    Ostream,
    Iostream,
}
impl Abbreviation {
    #[must_use]
    pub fn from_word(word: &[u8]) -> Option<Self> {
        Some(match word {
            b"Sa" => Self::Allocator,
            b"Sb" => Self::BasicString,
            b"Ss" => Self::String,
            b"Si" => Self::Istream,
            b"So" => Self::Ostream,
            b"Sd" => Self::Iostream,
            _ => return None,
        })
    }
    pub(crate) fn code(self) -> &'static [u8] {
        match self {
            Self::Allocator => b"Sa",
            Self::BasicString => b"Sb",
            Self::String => b"Ss",
            Self::Istream => b"Si",
            Self::Ostream => b"So",
            Self::Iostream => b"Sd",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    Pointer,
    LvalueReference,
    RvalueReference,
    PackExpansion,
    Qualified(Qualifiers),
    Vendor(NodeId),
    Array(u64),
    UnknownArray,
    ExpressionArray(NodeId),
}

/// Parameter kind participates in identity even when its encoding is the same.
/// These are the four `AbiGraph::template_param` kinds in the frozen producer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParameterKind {
    Type,
    TypePack,
    Template,
    TemplatePack,
}

/// Lists are held by the graph: tags for names/abbreviations, arguments for
/// templates/packs/vendor types, and parameters for function nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    None,
    Text,
    Std,
    FunctionScope {
        encoding: NodeId,
        /// Zero for an ordinary scope; otherwise default parameter index plus one.
        default_argument: u32,
    },
    RawFunctionScope(NodeId),
    Name {
        parent: Option<NodeId>,
        terminal: NodeId,
        internal: bool,
        /// Inline namespace tags participate in derivation but are not encoded.
        hidden_tags: bool,
        /// A closure prefix's initializing member: `<name> M` (5.1.8).
        data_member: bool,
        /// Number of earlier entities with this name directly in the function.
        occurrence: u32,
    },
    UnnamedType(u32),
    DecompositionName,
    ClosureName {
        /// Zero when omitted; otherwise the Itanium closure number plus one.
        number: u32,
        variadic: bool,
    },
    Terminal {
        kind: Terminal,
        inherited: Option<NodeId>,
    },
    Conversion(NodeId),
    LiteralOperator(NodeId),
    Template(NodeId),
    Abbreviation(Abbreviation),
    Builtin(Builtin),
    Modified {
        modifier: Modifier,
        operand: NodeId,
    },
    VendorType(NodeId),
    FunctionType {
        result: NodeId,
        owner: Option<NodeId>,
        qualifiers: Qualifiers,
        variadic: bool,
    },
    MemberPointer {
        owner: NodeId,
        member: NodeId,
    },
    TemplateParam {
        index: u32,
        level: u32,
        substitutable: bool,
        kind: ParameterKind,
    },
    Decltype {
        expression: NodeId,
        id_expression: bool,
    },
    FunctionParam {
        index: u32,
        level: u32,
    },
    ThisParam,
    Operation(Operation),
    Call,
    ConversionList(NodeId),
    BracedList(Option<NodeId>),
    ParenthesizedCall,
    PrefixStep {
        increment: bool,
    },
    AlignofType(NodeId),
    Cast {
        operation: Operation,
        ty: NodeId,
        operand: NodeId,
    },
    UnresolvedName(NodeId),
    VendorExpression(NodeId),
    SizeofType(NodeId),
    UnresolvedPrefix {
        root: Option<NodeId>,
        global: bool,
    },
    ScopedMember {
        owner: NodeId,
        member: NodeId,
        close_qualifier: bool,
    },
    MemberAccess {
        operation: Operation,
        object: NodeId,
        member: NodeId,
    },
    EntityReference(NodeId),
    ExpressionArgument(NodeId),
    Literal {
        ty: NodeId,
        bits: u64,
    },
    FloatingLiteral {
        ty: NodeId,
        bits: u64,
    },
    NonTypeParameter(NodeId),
    TypeParameter,
    TemplateParameter,
    ParameterPack(NodeId),
    TypedArgument {
        // Further parameter declarations live in this node's item slice.
        declaration: NodeId,
        argument: NodeId,
    },
    ArgumentPack,
    EntityArgument {
        entity: NodeId,
        address: bool,
    },
    ExternalSymbol(NodeId),
    Function {
        name: NodeId,
        result: Option<NodeId>,
        qualifiers: Qualifiers,
        variadic: bool,
    },
}

#[derive(Clone, Copy, Default)]
struct Span {
    start: u32,
    end: u32,
}
impl Span {
    fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
    fn append<T: Copy>(store: &mut Vec<T>, items: &[T]) -> Result<Self, Error> {
        let start = u32::try_from(store.len()).map_err(|_| Error::StorageLimit)?;
        let end = store
            .len()
            .checked_add(items.len())
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(Error::StorageLimit)?;
        store.extend_from_slice(items);
        Ok(Self { start, end })
    }
}
struct StoredNode {
    node: Node,
    items: Span,
    text: Span,
    next: Option<NodeId>,
}

/// Published nodes are immutable. Hash collisions compare their typed edges
/// and owner slices once at construction; substitution compares only IDs.
pub struct Graph {
    nodes: Vec<StoredNode>,
    items: Vec<NodeId>,
    text: Vec<u8>,
    index: HashMap<u64, NodeId>,
    pub(super) tags: super::tags::Tags,
}
impl Default for Graph {
    fn default() -> Self {
        Self {
            nodes: vec![StoredNode {
                node: Node::None,
                items: Span::default(),
                text: Span::default(),
                next: None,
            }],
            items: Vec::new(),
            text: Vec::new(),
            index: HashMap::new(),
            tags: super::tags::Tags::default(),
        }
    }
}
impl Graph {
    pub(crate) fn get(&self, id: NodeId) -> Node {
        self.nodes[id.index()].node
    }
    pub(crate) fn items(&self, id: NodeId) -> &[NodeId] {
        &self.items[self.nodes[id.index()].items.range()]
    }
    pub(crate) fn text(&self, id: NodeId) -> &[u8] {
        &self.text[self.nodes[id.index()].text.range()]
    }
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }
    /// Reserved payload bytes in graph stores and the intern index. Hash table
    /// control bytes and allocator overhead are outside this storage census.
    pub(crate) fn bytes_reserved(&self) -> usize {
        self.nodes.capacity() * size_of::<StoredNode>()
            + self.items.capacity() * size_of::<NodeId>()
            + self.text.capacity()
            + self.index.capacity() * size_of::<(u64, NodeId)>()
            + self.tags.bytes_reserved()
    }
    pub(crate) fn reset(&mut self) {
        self.nodes.truncate(1);
        self.items.clear();
        self.text.clear();
        self.index.clear();
        self.tags = super::tags::Tags::default();
    }
    pub(crate) fn intern(&mut self, node: Node, items: &[NodeId]) -> Result<NodeId, Error> {
        self.intern_with(node, items, &[])
    }
    pub(crate) fn string(&mut self, text: &[u8]) -> Result<NodeId, Error> {
        self.intern_with(Node::Text, &[], text)
    }
    fn intern_with(&mut self, node: Node, items: &[NodeId], text: &[u8]) -> Result<NodeId, Error> {
        let mut hasher = DefaultHasher::new();
        node.hash(&mut hasher);
        items.hash(&mut hasher);
        text.hash(&mut hasher);
        let hash = hasher.finish();
        let head = self.index.get(&hash).copied();
        let mut cursor = head;
        while let Some(id) = cursor {
            let stored = &self.nodes[id.index()];
            if stored.node == node && self.items(id) == items && self.text(id) == text {
                return Ok(id);
            }
            cursor = stored.next;
        }
        let id = NodeId(
            NonZeroU32::new(u32::try_from(self.nodes.len()).map_err(|_| Error::StorageLimit)?)
                .ok_or(Error::StorageLimit)?,
        );
        let items = Span::append(&mut self.items, items)?;
        let text = Span::append(&mut self.text, text)?;
        self.nodes.push(StoredNode {
            node,
            items,
            text,
            next: head,
        });
        self.index.insert(hash, id);
        Ok(id)
    }
    pub(crate) fn name(
        &mut self,
        parent: Option<NodeId>,
        terminal: NodeId,
        tags: &[NodeId],
    ) -> Result<NodeId, Error> {
        let mut sorted = tags.to_vec();
        sorted.sort_unstable_by(|a, b| self.text(*a).cmp(self.text(*b)));
        sorted.dedup();
        self.intern(
            Node::Name {
                parent,
                terminal,
                internal: false,
                hidden_tags: false,
                data_member: false,
                occurrence: 0,
            },
            &sorted,
        )
    }
    fn is_std_name(&self, name: NodeId, spelling: &[u8]) -> bool {
        matches!(self.get(name), Node::Name {
            parent: Some(parent), terminal, internal: false, hidden_tags: false, data_member: false, occurrence: 0,
        } if self.get(parent) == Node::Std
            && self.get(terminal) == Node::Text
            && self.text(terminal) == spelling)
    }
    fn is_char_argument_of(&self, ty: NodeId, abbreviation: Option<Abbreviation>) -> bool {
        let Node::Template(template) = self.get(ty) else {
            return false;
        };
        let [argument] = self.items(ty) else {
            return false;
        };
        self.get(*argument) == Node::Builtin(Builtin::Char)
            && self.items(template).is_empty()
            && abbreviation.map_or_else(
                || self.is_std_name(template, b"char_traits"),
                |kind| self.get(template) == Node::Abbreviation(kind),
            )
    }
    /// Compiler class names use the ABI's standard catalog. Explicit fact
    /// producers keep using `intern(Template(..))` to choose their own form.
    pub(crate) fn abbreviated_template_id(
        &mut self,
        template: NodeId,
        arguments: &[NodeId],
    ) -> Result<NodeId, Error> {
        let char_traits = arguments.len() >= 2
            && self.get(arguments[0]) == Node::Builtin(Builtin::Char)
            && self.is_char_argument_of(arguments[1], None);
        let (kind, whole) = if self.is_std_name(template, b"allocator") {
            (Some(Abbreviation::Allocator), false)
        } else if self.is_std_name(template, b"basic_string") {
            let whole = arguments.len() == 3
                && char_traits
                && self.is_char_argument_of(arguments[2], Some(Abbreviation::Allocator));
            (
                Some(if whole {
                    Abbreviation::String
                } else {
                    Abbreviation::BasicString
                }),
                whole,
            )
        } else if arguments.len() == 2 && char_traits {
            let kind = if self.is_std_name(template, b"basic_istream") {
                Some(Abbreviation::Istream)
            } else if self.is_std_name(template, b"basic_ostream") {
                Some(Abbreviation::Ostream)
            } else if self.is_std_name(template, b"basic_iostream") {
                Some(Abbreviation::Iostream)
            } else {
                None
            };
            (kind, true)
        } else {
            (None, false)
        };
        let Some(kind) = kind else {
            return self.intern(Node::Template(template), arguments);
        };
        let tags = self.items(template).to_vec();
        let abbreviation = self.intern(Node::Abbreviation(kind), &tags)?;
        if whole {
            Ok(abbreviation)
        } else {
            self.intern(Node::Template(abbreviation), arguments)
        }
    }
    pub(crate) fn tagged(&mut self, id: NodeId, tags: &[NodeId]) -> Result<NodeId, Error> {
        let node = self.get(id);
        if let Node::Template(template) = node {
            let args = self.items(id).to_vec();
            let template = self.tagged(template, tags)?;
            return self.intern(Node::Template(template), &args);
        }
        if !matches!(node, Node::Name { .. } | Node::Abbreviation(_)) {
            return Err(Error::diagnostic(
                "tagged type needs a named or class-template type",
            ));
        }
        let mut merged = self.items(id).to_vec();
        merged.extend_from_slice(tags);
        merged.sort_unstable_by(|a, b| self.text(*a).cmp(self.text(*b)));
        merged.dedup();
        self.intern(node, &merged)
    }
    pub(crate) fn modified(
        &mut self,
        mut modifier: Modifier,
        mut operand: NodeId,
    ) -> Result<NodeId, Error> {
        if let Modifier::Qualified(mut cv) = modifier {
            if let Node::FunctionType {
                result,
                owner,
                qualifiers,
                variadic,
            } = self.get(operand)
            {
                let params = self.items(operand).to_vec();
                return self.intern(
                    Node::FunctionType {
                        result,
                        owner,
                        qualifiers: qualifiers.union(cv),
                        variadic,
                    },
                    &params,
                );
            }
            if let Node::Modified {
                modifier: Modifier::Qualified(previous),
                operand: base,
            } = self.get(operand)
            {
                cv = cv.union(previous);
                operand = base;
            }
            modifier = Modifier::Qualified(cv);
        }
        self.intern(Node::Modified { modifier, operand }, &[])
    }
    pub(crate) fn member_pointer(
        &mut self,
        owner: NodeId,
        mut member: NodeId,
    ) -> Result<NodeId, Error> {
        if let Node::FunctionType {
            result,
            qualifiers,
            variadic,
            ..
        } = self.get(member)
        {
            let params = self.items(member).to_vec();
            member = self.intern(
                Node::FunctionType {
                    result,
                    owner: Some(owner),
                    qualifiers,
                    variadic,
                },
                &params,
            )?;
        }
        self.intern(Node::MemberPointer { owner, member }, &[])
    }
    pub(crate) fn drop_void(&self, params: &mut Vec<NodeId>) {
        if params.len() == 1 && self.get(params[0]) == Node::Builtin(Builtin::Void) {
            params.clear();
        }
    }
}
