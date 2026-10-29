// Translated from preprocess/macro/hide_sets.{h,cpp}. See NOTICE.
//! Interned ordered additions, logarithmic ancestor unions and lazy persistent membership tries.
use super::super::error::PreprocessError;
use crate::lex::identifiers::NameId;
use crate::support::hash::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct HideSetId(u32);
impl HideSetId {
    fn index(self) -> usize {
        self.0 as usize
    }
}
struct Node {
    parent: HideSetId,
    depth: u32,
    jump: HideSetId,
    name: Option<NameId>,
    summary: u64,
    index: Option<TrieRef>,
}
#[derive(Clone, Copy)]
enum TrieRef {
    Empty,
    Leaf(NameId),
    Branch(u32),
}
#[derive(Clone, Copy)]
struct Branch {
    children: [TrieRef; 2],
}

pub(crate) struct HideSets {
    nodes: Vec<Node>,
    additions: HashMap<(HideSetId, NameId), HideSetId>,
    branches: Vec<Branch>,
    merged: Option<(HideSetId, HideSetId, HideSetId)>,
}
impl Default for HideSets {
    fn default() -> Self {
        Self {
            nodes: vec![Node {
                parent: HideSetId(0),
                depth: 0,
                jump: HideSetId(0),
                name: None,
                summary: 0,
                index: Some(TrieRef::Empty),
            }],
            additions: HashMap::default(),
            branches: Vec::new(),
            merged: None,
        }
    }
}
impl HideSets {
    pub(crate) fn add(
        &mut self,
        set: HideSetId,
        name: NameId,
    ) -> Result<HideSetId, PreprocessError> {
        if let Some(&id) = self.additions.get(&(set, name)) {
            return Ok(id);
        }
        let parent = &self.nodes[set.index()];
        let parent_jump = &self.nodes[parent.jump.index()];
        let jump = if set.0 != 0
            && parent.depth - parent_jump.depth
                == parent_jump.depth - self.nodes[parent_jump.jump.index()].depth
        {
            parent_jump.jump
        } else {
            set
        };
        let id = HideSetId(
            u32::try_from(self.nodes.len())
                .map_err(|_| PreprocessError::Limit("too many macro hide sets"))?,
        );
        let depth = parent
            .depth
            .checked_add(1)
            .ok_or(PreprocessError::Limit("macro hide set too deep"))?;
        let summary = parent.summary | (1 << (name.bits() & 63));
        self.nodes.push(Node {
            parent: set,
            depth,
            jump,
            name: Some(name),
            summary,
            index: None,
        });
        self.additions.insert((set, name), id);
        Ok(id)
    }
    pub(crate) fn without_last(&self, set: HideSetId) -> HideSetId {
        self.nodes[set.index()].parent
    }
    pub(crate) fn contains(
        &mut self,
        mut set: HideSetId,
        name: NameId,
    ) -> Result<bool, PreprocessError> {
        let bit = 1 << (name.bits() & 63);
        let mut steps = 0;
        while set.0 != 0 && self.nodes[set.index()].summary & bit != 0 {
            let node = &self.nodes[set.index()];
            if node.name == Some(name) {
                return Ok(true);
            }
            if steps == 32 {
                let mut reference = self.index_of(set)?;
                let mut level = 0;
                while let TrieRef::Branch(id) = reference {
                    reference =
                        self.branches[id as usize].children[((name.bits() >> level) & 1) as usize];
                    level += 1;
                }
                return Ok(matches!(reference, TrieRef::Leaf(id) if id == name));
            }
            set = node.parent;
            steps += 1;
        }
        Ok(false)
    }
    pub(crate) fn merge(
        &mut self,
        mut set: HideSetId,
        mut other: HideSetId,
    ) -> Result<HideSetId, PreprocessError> {
        if other.0 == 0 || other == set {
            return Ok(set);
        }
        if set.0 == 0 {
            return Ok(other);
        }
        if let Some((a, b, result)) = self.merged
            && a == set
            && b == other
        {
            return Ok(result);
        }
        let key = (set, other);
        let shared = self.common_ancestor(set, other);
        if shared == set {
            self.merged = Some((key.0, key.1, other));
            return Ok(other);
        }
        let mut names = Vec::new();
        while other != shared {
            let node = &self.nodes[other.index()];
            if let Some(name) = node.name {
                names.push(name);
            }
            other = node.parent;
        }
        for name in names.into_iter().rev() {
            if !self.contains(set, name)? {
                set = self.add(set, name)?;
            }
        }
        self.merged = Some((key.0, key.1, set));
        Ok(set)
    }
    fn ancestor_at(&self, mut set: HideSetId, depth: u32) -> HideSetId {
        while self.nodes[set.index()].depth > depth {
            let node = &self.nodes[set.index()];
            set = if self.nodes[node.jump.index()].depth >= depth {
                node.jump
            } else {
                node.parent
            };
        }
        set
    }
    fn common_ancestor(&self, mut a: HideSetId, mut b: HideSetId) -> HideSetId {
        a = self.ancestor_at(a, self.nodes[b.index()].depth);
        b = self.ancestor_at(b, self.nodes[a.index()].depth);
        while a != b {
            let x = &self.nodes[a.index()];
            let y = &self.nodes[b.index()];
            if x.jump == y.jump {
                a = x.parent;
                b = y.parent;
            } else {
                a = x.jump;
                b = y.jump;
            }
        }
        a
    }
    fn index_of(&mut self, mut set: HideSetId) -> Result<TrieRef, PreprocessError> {
        let mut path = Vec::new();
        while self.nodes[set.index()].index.is_none() {
            path.push(set);
            set = self.nodes[set.index()].parent;
        }
        let mut root = self.nodes[set.index()]
            .index
            .ok_or(PreprocessError::Invariant("missing hide set root"))?;
        for id in path.into_iter().rev() {
            let name = self.nodes[id.index()]
                .name
                .ok_or(PreprocessError::Invariant("missing hide set name"))?;
            root = self.index_insert(root, name, 0)?;
            self.nodes[id.index()].index = Some(root);
        }
        Ok(root)
    }
    // Bounded by the 32 bits of an identifier; the other branch remains shared.
    fn index_insert(
        &mut self,
        reference: TrieRef,
        name: NameId,
        level: u32,
    ) -> Result<TrieRef, PreprocessError> {
        let mut branch = match reference {
            TrieRef::Empty => return Ok(TrieRef::Leaf(name)),
            TrieRef::Leaf(id) if id == name => return Ok(reference),
            TrieRef::Leaf(id) => {
                let mut branch = Branch {
                    children: [TrieRef::Empty; 2],
                };
                branch.children[((id.bits() >> level) & 1) as usize] = reference;
                branch
            }
            TrieRef::Branch(id) => self.branches[id as usize],
        };
        let bit = ((name.bits() >> level) & 1) as usize;
        branch.children[bit] = self.index_insert(branch.children[bit], name, level + 1)?;
        let id = u32::try_from(self.branches.len())
            .map_err(|_| PreprocessError::Limit("macro hide set index too large"))?;
        self.branches.push(branch);
        Ok(TrieRef::Branch(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::identifiers::Identifiers;

    #[test]
    fn deep_membership_union_and_last_add_preserve_token_history() {
        let mut identifiers = Identifiers::default();
        let names: Vec<_> = (0..320)
            .map(|index| {
                identifiers
                    .intern(format!("macro_{index}").as_bytes())
                    .unwrap()
            })
            .collect();
        let mut sets = HideSets::default();
        let mut prefixes = vec![HideSetId::default()];
        for &name in &names[..256] {
            let before = *prefixes.last().unwrap();
            let after = sets.add(before, name).unwrap();
            assert_eq!(sets.add(before, name).unwrap(), after);
            assert_eq!(sets.without_last(after), before);
            prefixes.push(after);
        }
        let deep = prefixes[256];
        // Membership beyond the 32-step walk forces persistent index creation.
        for (index, &name) in names.iter().enumerate() {
            assert_eq!(sets.contains(deep, name).unwrap(), index < 256);
        }
        let mut other = prefixes[128];
        for &name in &names[192..] {
            other = sets.add(other, name).unwrap();
        }
        let merged = sets.merge(deep, other).unwrap();
        for &name in &names {
            assert!(sets.contains(merged, name).unwrap());
        }
        assert_eq!(sets.merge(deep, prefixes[128]).unwrap(), deep);
        assert_eq!(sets.merge(prefixes[128], deep).unwrap(), deep);
        // Unions do not mutate an earlier token's set.
        assert!(!sets.contains(deep, names[319]).unwrap());
        let duplicate = sets.add(deep, names[0]).unwrap();
        assert_eq!(sets.without_last(duplicate), deep);
    }
}
