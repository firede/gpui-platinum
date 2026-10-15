use std::{cmp::Ordering, time::SystemTime};

/// The kinds of items in the desk's in-memory disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeKind {
    Folder,
    Document,
    Application,
    Disk,
    ControlPanel,
}

impl NodeKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Document => "text document",
            Self::Application => "application",
            Self::Disk => "disk",
            Self::ControlPanel => "control panel",
        }
    }

    pub fn is_container(self) -> bool {
        matches!(self, Self::Folder | Self::Disk)
    }
}

/// An item of the tree. IDs stay the same when items move or are sorted.
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub id: usize,
    pub title: String,
    pub kind: NodeKind,
    pub text: String,
    pub modified: SystemTime,
    pub runs: Option<Vec<crate::Run>>,
    pub alignments: Option<Vec<crate::Alignment>>,
    pub children: Vec<TreeNode>,
    pub open: bool,
}

impl TreeNode {
    pub fn new(id: usize, title: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id,
            title: title.into(),
            kind,
            text: String::new(),
            modified: SystemTime::now(),
            runs: None,
            alignments: None,
            children: Vec::new(),
            open: false,
        }
    }

    pub fn size(&self) -> usize {
        match self.kind {
            NodeKind::Document => self.text.len(),
            NodeKind::Application | NodeKind::ControlPanel => 24 * 1024 + self.title.len() * 512,
            _ => self.children.iter().map(Self::size).sum(),
        }
    }

    pub fn write_document(
        &mut self,
        text: String,
        runs: Vec<crate::Run>,
        alignments: Vec<crate::Alignment>,
    ) {
        self.text = text;
        self.runs = Some(runs);
        self.alignments = Some(alignments);
        self.modified = SystemTime::now();
    }

    pub fn find(&self, id: usize) -> Option<&Self> {
        if self.id == id {
            Some(self)
        } else {
            self.children.iter().find_map(|child| child.find(id))
        }
    }

    pub fn find_mut(&mut self, id: usize) -> Option<&mut Self> {
        if self.id == id {
            Some(self)
        } else {
            self.children
                .iter_mut()
                .find_map(|child| child.find_mut(id))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TreeRow<'a> {
    pub node: &'a TreeNode,
    pub depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Size,
    Kind,
}

/// The tree view's selection and the files shared by the desktop applications.
#[derive(Clone, Debug)]
pub struct FileTree {
    pub roots: Vec<TreeNode>,
    pub selected: Option<usize>,
    next_id: usize,
}

impl Default for FileTree {
    fn default() -> Self {
        Self::sample()
    }
}

impl FileTree {
    pub fn new(roots: Vec<TreeNode>) -> Self {
        fn largest(nodes: &[TreeNode]) -> usize {
            nodes
                .iter()
                .map(|node| node.id.max(largest(&node.children)))
                .max()
                .unwrap_or(0)
        }
        Self {
            next_id: largest(&roots).saturating_add(1),
            roots,
            selected: None,
        }
    }

    pub fn sample() -> Self {
        let mut tree = Self::new(vec![TreeNode::new(0, "Platinum Disk", NodeKind::Disk)]);
        tree.roots[0].open = true;
        let apps = tree.add_folder(0, "Applications").unwrap();
        for name in [
            "Alarm Clock",
            "Calculator",
            "Controls",
            "Jotter",
            "Ledger",
            "Puzzle",
            "Scribe",
        ] {
            tree.add_node(apps, name, NodeKind::Application, "")
                .unwrap();
        }
        let documents = tree.add_folder(0, "Documents").unwrap();
        tree.add_document(documents, "Letter", "Dear Ada,\n\nThe new desk is up and running. Every window has a striped title bar again, and the buttons press in when you click them.\n\nWrite soon,\nGrace");
        tree.add_document(documents, "Recipes", "Pancakes\n- 2 cups flour\n- 2 eggs\n- 1 1/2 cups milk\n- a pinch of salt\n\nWhisk, rest ten minutes, fry in butter.");
        tree.add_document(
            documents,
            "To Do",
            "- Water the plants\n- Back up the disk\n- Finish the puzzle",
        );
        let system = tree.add_folder(0, "System Folder").unwrap();
        let panels = tree.add_folder(system, "Control Panels").unwrap();
        for name in ["Appearance", "Date & Time", "Language", "Mouse", "Sound"] {
            tree.add_node(panels, name, NodeKind::ControlPanel, "")
                .unwrap();
        }
        tree.add_document(0, "Read Me", "Welcome to the Platinum desk.\n\nDouble-click an icon to open it. Drag windows by their title bars, resize them from the bottom right corner, and roll them up with the box at the right of the title bar.\n\nThe Appearance control panel changes the accent color and the desktop pattern.");
        tree
    }

    pub fn find(&self, id: usize) -> Option<&TreeNode> {
        self.roots.iter().find_map(|node| node.find(id))
    }

    pub fn find_mut(&mut self, id: usize) -> Option<&mut TreeNode> {
        self.roots.iter_mut().find_map(|node| node.find_mut(id))
    }

    pub fn visible_rows(&self) -> Vec<TreeRow<'_>> {
        fn append<'a>(nodes: &'a [TreeNode], depth: usize, rows: &mut Vec<TreeRow<'a>>) {
            for node in nodes {
                rows.push(TreeRow { node, depth });
                if node.open {
                    append(&node.children, depth + 1, rows);
                }
            }
        }
        let mut rows = Vec::new();
        append(&self.roots, 0, &mut rows);
        rows
    }

    pub fn select(&mut self, id: usize) -> bool {
        if self.find(id).is_none() {
            return false;
        }
        self.selected = Some(id);
        true
    }

    pub fn toggle(&mut self, id: usize) {
        if let Some(node) = self.find_mut(id)
            && !node.children.is_empty()
        {
            node.open = !node.open;
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let rows = self.visible_rows();
        if rows.is_empty() {
            self.selected = None;
            return;
        }
        let current = rows
            .iter()
            .position(|row| Some(row.node.id) == self.selected);
        let next = match current {
            Some(index) => index
                .saturating_add_signed(delta as isize)
                .min(rows.len() - 1),
            None if delta < 0 => rows.len() - 1,
            None => 0,
        };
        self.selected = Some(rows[next].node.id);
    }

    pub fn right(&mut self) {
        let Some(id) = self.selected else {
            self.move_selection(1);
            return;
        };
        let Some(node) = self.find_mut(id) else {
            return;
        };
        if node.open {
            if let Some(child) = node.children.first() {
                self.selected = Some(child.id);
            }
        } else if !node.children.is_empty() {
            node.open = true;
        }
    }

    pub fn left(&mut self) {
        let Some(id) = self.selected else { return };
        let Some(node) = self.find_mut(id) else {
            return;
        };
        if node.open && !node.children.is_empty() {
            node.open = false;
        } else {
            let path = self.path(id);
            if path.len() >= 2 {
                self.selected = Some(path[path.len() - 2]);
            }
        }
    }

    /// Choose the next visible item beginning with the text, wrapping at the end.
    pub fn select_prefix(&mut self, prefix: &str) -> bool {
        let prefix = prefix.to_lowercase();
        let rows = self.visible_rows();
        let start = rows
            .iter()
            .position(|row| Some(row.node.id) == self.selected)
            .map(|index| index + 1)
            .unwrap_or(0);
        let selected = (0..rows.len())
            .map(|offset| &rows[(start + offset) % rows.len()])
            .find(|row| row.node.title.to_lowercase().starts_with(&prefix))
            .map(|row| row.node.id);
        if let Some(selected) = selected {
            self.selected = Some(selected);
            true
        } else {
            false
        }
    }

    /// The disk-to-item path, including the item itself.
    pub fn path(&self, id: usize) -> Vec<usize> {
        fn search(nodes: &[TreeNode], id: usize, path: &mut Vec<usize>) -> bool {
            for node in nodes {
                path.push(node.id);
                if node.id == id || search(&node.children, id, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut path = Vec::new();
        search(&self.roots, id, &mut path);
        path
    }

    pub fn path_name(&self, id: usize) -> String {
        self.path(id)
            .iter()
            .filter_map(|id| self.find(*id))
            .map(|node| node.title.as_str())
            .collect::<Vec<_>>()
            .join(":")
    }

    pub fn children_sorted(
        &self,
        parent: usize,
        column: SortColumn,
        descending: bool,
    ) -> Vec<&TreeNode> {
        let mut nodes: Vec<_> = self
            .find(parent)
            .map(|node| node.children.iter().collect())
            .unwrap_or_default();
        nodes.sort_by(|left, right| {
            let order = compare_nodes(left, right, column);
            if descending { order.reverse() } else { order }
        });
        nodes
    }

    pub fn add_folder(&mut self, parent: usize, title: impl Into<String>) -> Option<usize> {
        self.add_node(parent, title, NodeKind::Folder, "")
    }

    pub fn add_document(
        &mut self,
        parent: usize,
        title: impl Into<String>,
        text: impl Into<String>,
    ) -> Option<usize> {
        self.add_node(parent, title, NodeKind::Document, text)
    }

    fn add_node(
        &mut self,
        parent: usize,
        title: impl Into<String>,
        kind: NodeKind,
        text: impl Into<String>,
    ) -> Option<usize> {
        if !self.find(parent)?.kind.is_container() {
            return None;
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1)?;
        let mut node = TreeNode::new(id, title, kind);
        node.text = text.into();
        let parent = self.find_mut(parent)?;
        parent.children.push(node);
        parent
            .children
            .sort_by(|a, b| compare_nodes(a, b, SortColumn::Name));
        Some(id)
    }

    pub fn remove(&mut self, id: usize) -> Option<TreeNode> {
        fn remove_from(nodes: &mut Vec<TreeNode>, id: usize) -> Option<TreeNode> {
            if let Some(index) = nodes.iter().position(|node| node.id == id) {
                return Some(nodes.remove(index));
            }
            nodes
                .iter_mut()
                .find_map(|node| remove_from(&mut node.children, id))
        }
        let node = remove_from(&mut self.roots, id)?;
        if self
            .selected
            .is_some_and(|selected| node.find(selected).is_some())
        {
            self.selected = None;
        }
        Some(node)
    }

    /// Move an item, rejecting drops into itself or into one of its descendants.
    pub fn move_node(&mut self, id: usize, parent: usize) -> bool {
        let Some(target) = self.find(parent) else {
            return false;
        };
        if !target.kind.is_container() || self.path(parent).contains(&id) {
            return false;
        }
        if self.roots.iter().any(|node| node.id == id) {
            return false;
        }
        let selected = self.selected;
        let Some(node) = self.remove(id) else {
            return false;
        };
        let target = self
            .find_mut(parent)
            .expect("the destination is outside the item moved");
        target.children.push(node);
        target
            .children
            .sort_by(|a, b| compare_nodes(a, b, SortColumn::Name));
        self.selected = selected;
        true
    }

    /// Copy an item with fresh IDs, preserving its descendants and contents.
    pub fn duplicate(
        &mut self,
        id: usize,
        parent: usize,
        title: impl Into<String>,
    ) -> Option<usize> {
        if !self.find(parent)?.kind.is_container() {
            return None;
        }
        let mut node = self.find(id)?.clone();
        fn assign_ids(node: &mut TreeNode, next: &mut usize) -> Option<()> {
            node.id = *next;
            *next = next.checked_add(1)?;
            for child in &mut node.children {
                assign_ids(child, next)?;
            }
            Some(())
        }
        assign_ids(&mut node, &mut self.next_id)?;
        node.title = title.into();
        node.modified = SystemTime::now();
        let id = node.id;
        self.find_mut(parent)?.children.push(node);
        Some(id)
    }
}

fn compare_nodes(left: &TreeNode, right: &TreeNode, column: SortColumn) -> Ordering {
    let by_name = || left.title.to_lowercase().cmp(&right.title.to_lowercase());
    match column {
        SortColumn::Name => by_name(),
        SortColumn::Size => left.size().cmp(&right.size()).then_with(by_name),
        SortColumn::Kind => left.kind.name().cmp(right.kind.name()).then_with(by_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saving_document_content_updates_its_modified_time() {
        let mut node = TreeNode::new(1, "Letter", NodeKind::Document);
        node.modified = SystemTime::UNIX_EPOCH;
        let before_save = SystemTime::now();
        node.write_document(
            "Saved text".into(),
            Vec::new(),
            vec![crate::Alignment::Center],
        );
        assert_eq!(node.text, "Saved text");
        assert_eq!(
            node.alignments.as_deref(),
            Some([crate::Alignment::Center].as_slice())
        );
        assert!(node.modified >= before_save);
    }
    #[test]
    fn duplicates_get_a_fresh_modified_time_without_changing_the_original() {
        let mut tree = FileTree::new(vec![TreeNode::new(0, "Disk", NodeKind::Disk)]);
        let original = tree.add_document(0, "Letter", "Hello").unwrap();
        tree.find_mut(original).unwrap().modified = SystemTime::UNIX_EPOCH;
        let before_copy = SystemTime::now();
        let copy = tree.duplicate(original, 0, "Letter copy").unwrap();
        assert_ne!(copy, original);
        assert_eq!(
            tree.find(original).unwrap().modified,
            SystemTime::UNIX_EPOCH
        );
        let node = tree.find(copy).unwrap();
        assert!(node.modified >= before_copy);
        assert_eq!(node.text, "Hello");
    }

    fn id(tree: &FileTree, title: &str) -> usize {
        fn search(node: &TreeNode, title: &str) -> Option<usize> {
            if node.title == title {
                Some(node.id)
            } else {
                node.children.iter().find_map(|child| search(child, title))
            }
        }
        tree.roots
            .iter()
            .find_map(|node| search(node, title))
            .unwrap()
    }

    #[test]
    fn arrows_open_descend_close_and_return_to_the_parent() {
        let mut tree = FileTree::sample();
        let docs = id(&tree, "Documents");
        let letter = id(&tree, "Letter");
        tree.select(docs);
        tree.right();
        assert_eq!(tree.selected, Some(docs));
        assert!(
            tree.visible_rows()
                .iter()
                .any(|row| row.node.id == letter && row.depth == 2)
        );
        tree.right();
        assert_eq!(tree.selected, Some(letter));
        tree.left();
        assert_eq!(tree.selected, Some(docs));
        tree.left();
        assert!(!tree.find(docs).unwrap().open);
    }

    #[test]
    fn sorting_uses_numeric_sizes_without_changing_selection_or_ids() {
        let mut tree = FileTree::sample();
        let docs = id(&tree, "Documents");
        let small = tree.add_document(docs, "z", "123").unwrap();
        let large = tree.add_document(docs, "a", "1234567890").unwrap();
        tree.select(small);
        let rows = tree.children_sorted(docs, SortColumn::Size, false);
        assert_eq!(rows[0].id, small);
        assert_eq!(rows[1].id, large);
        assert_eq!(tree.selected, Some(small));
    }

    #[test]
    fn navigation_and_prefix_search_reach_the_visible_items() {
        let mut tree = FileTree::sample();
        tree.move_selection(1);
        assert_eq!(tree.selected, Some(0));
        assert!(tree.select_prefix("doc"));
        assert_eq!(tree.selected, Some(id(&tree, "Documents")));
        tree.move_selection(100);
        assert_eq!(tree.selected, Some(id(&tree, "System Folder")));
    }

    #[test]
    fn moves_preserve_contents_and_reject_cycles() {
        let mut tree = FileTree::sample();
        let docs = id(&tree, "Documents");
        let letter = id(&tree, "Letter");
        let folder = tree.add_folder(docs, "Archive").unwrap();
        tree.select(letter);
        assert!(!tree.move_node(docs, folder));
        assert!(tree.move_node(letter, folder));
        assert_eq!(
            tree.path_name(letter),
            "Platinum Disk:Documents:Archive:Letter"
        );
        assert_eq!(tree.selected, Some(letter));
        assert!(tree.find(letter).unwrap().text.starts_with("Dear Ada,"));
        tree.remove(folder);
        assert!(tree.find(letter).is_none());
        assert_eq!(tree.selected, None);
    }
}
