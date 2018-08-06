use std::fmt;
use std::fs::Metadata;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct Node {
    path: PathBuf,
    metadata: Metadata,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
struct NodeId(usize);

struct Arena(Vec<Node>);

impl Arena {
    fn add(&mut self, node: Node) -> NodeId {
        let idx = self.0.len();
        self.0.push(node);
        NodeId(idx)
    }

    #[inline]
    fn at(&self, id: NodeId) -> &Node {
        &self.0[id.0]
    }

    #[inline]
    fn at_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.0[id.0]
    }
}

#[derive(Clone)]
pub struct File {
    arena: Rc<Arena>,
    id: NodeId,
}

impl File {
    #[inline]
    fn get(&self) -> &Node {
        &self.arena.at(self.id)
    }

    #[inline]
    pub fn path(&self) -> &Path {
        &self.get().path
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.path()
            .file_name()
            .and_then(|p| p.to_str())
            .unwrap_or("")
    }

    #[inline]
    pub fn stem(&self) -> &str {
        self.path()
            .file_stem()
            .and_then(|p| p.to_str())
            .unwrap_or("")
    }

    #[inline]
    pub fn extension(&self) -> Option<&str> {
        self.path().extension().and_then(|p| p.to_str())
    }

    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.get().metadata
    }

    #[inline]
    pub fn is_dir(&self) -> bool {
        self.metadata().is_dir()
    }

    #[inline]
    pub fn is_file(&self) -> bool {
        self.metadata().is_file()
    }

    #[inline]
    pub fn parent(&self) -> Option<File> {
        self.get().parent.map(|parent_id| File {
            arena: self.arena.clone(),
            id: parent_id,
        })
    }

    #[inline]
    pub fn children(&self) -> ChildrenIter {
        ChildrenIter {
            node_id: self.id,
            pos: 0,
            len: self.get().children.len(),
            arena: self.arena.clone(),
        }
    }

    #[inline]
    pub fn siblings(&self) -> Option<SiblingsIter> {
        self.parent().map(|parent| SiblingsIter {
            node_id: parent.id,
            creator: self.id,
            pos: 0,
            len: parent.get().children.len(),
            arena: self.arena.clone(),
        })
    }

    #[inline]
    pub fn descendants(&self) -> DescendantsIter {
        DescendantsIter {
            queue: self.get().children.iter().rev().cloned().collect(),
            arena: self.arena.clone(),
        }
    }

    pub fn name_contains(&self, pattern: &str) -> bool {
        let pattern = pattern.to_lowercase();
        let name = self.name().to_lowercase();
        name.contains(&pattern)
    }
}

impl fmt::Debug for File {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        w.debug_struct("File")
            .field("path", &self.path().display())
            .field("children", &self.children().collect::<Vec<_>>())
            .finish()
    }
}

impl PartialEq for File {
    fn eq(&self, other: &File) -> bool {
        self.id == other.id
    }
}

impl Eq for File {}

impl Hash for File {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.id.hash(state)
    }
}

pub struct ChildrenIter {
    node_id: NodeId,
    pos: usize,
    len: usize,
    arena: Rc<Arena>,
}

impl Iterator for ChildrenIter {
    type Item = File;

    fn next(&mut self) -> Option<File> {
        if self.pos < self.len {
            let child_id = self.arena.at(self.node_id).children[self.pos];
            self.pos += 1;
            Some(File {
                id: child_id,
                arena: self.arena.clone(),
            })
        } else {
            None
        }
    }
}

pub struct SiblingsIter {
    node_id: NodeId,
    creator: NodeId,
    pos: usize,
    len: usize,
    arena: Rc<Arena>,
}

impl Iterator for SiblingsIter {
    type Item = File;

    fn next(&mut self) -> Option<File> {
        while self.pos < self.len {
            let child_id = self.arena.at(self.node_id).children[self.pos];
            self.pos += 1;
            if child_id != self.creator {
                return Some(File {
                    id: child_id,
                    arena: self.arena.clone(),
                });
            }
        }
        None
    }
}

pub struct DescendantsIter {
    queue: Vec<NodeId>,
    arena: Rc<Arena>,
}

impl Iterator for DescendantsIter {
    type Item = File;

    fn next(&mut self) -> Option<File> {
        if let Some(id) = self.queue.pop() {
            let node = self.arena.at(id);
            if node.metadata.is_dir() {
                self.queue.extend(node.children.iter().rev().cloned());
            }
            return Some(File {
                id: id,
                arena: self.arena.clone(),
            });
        }
        None
    }
}

pub fn walk(root: impl AsRef<Path>) -> io::Result<File> {
    let root = root.as_ref();
    let mut arena = Arena(Vec::new());

    let node = Node {
        path: root.to_owned(),
        metadata: root.metadata()?,
        parent: None,
        children: vec![],
    };

    let id = arena.add(node);

    walk_rec(root, &mut arena, id)?;

    Ok(File {
        id: id,
        arena: Rc::new(arena),
    })
}

fn walk_rec(parent_path: &Path, arena: &mut Arena, parent_id: NodeId) -> io::Result<()> {
    for entry in parent_path.read_dir()? {
        let entry = entry?;
        let path = entry.path();

        let node = Node {
            path: path.clone(),
            metadata: entry.metadata()?,
            parent: Some(parent_id),
            children: vec![],
        };

        let is_dir = node.metadata.is_dir();
        let id = arena.add(node);
        arena.at_mut(parent_id).children.push(id);

        if is_dir {
            walk_rec(&path, arena, id)?;
        }
    }

    Ok(())
}
