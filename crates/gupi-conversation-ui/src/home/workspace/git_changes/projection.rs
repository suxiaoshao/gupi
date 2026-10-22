//! Navigation derived only from source-qualified change records, never the filesystem.
use super::*;
use std::{collections::BTreeMap, ffi::OsString, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Directory {
    pub group: Group,
    pub path: PathBuf,
}
impl Directory {
    pub fn id(&self) -> String {
        format!(
            "git-directory-{}-{}",
            self.group.key(),
            identity(ChangeSource::Staged, &self.path)
        )
    }
    pub fn parent_id(&self) -> String {
        match self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            Some(path) => Self {
                group: self.group,
                path: path.into(),
            }
            .id(),
            None => format!("git-group-{}", self.group.key()),
        }
    }
}
#[derive(Default)]
struct Node<'a> {
    directories: BTreeMap<OsString, Node<'a>>,
    files: Vec<&'a Entry>,
    count: usize,
    stats: Option<(usize, usize)>,
}
impl<'a> Node<'a> {
    fn insert(&mut self, entry: &'a Entry, components: &[OsString]) {
        self.count += 1;
        self.stats = match (self.stats, entry.file.stats()) {
            (Some((a, r)), Some((x, y))) => Some((a + x, r + y)),
            _ => None,
        };
        if let Some((first, rest)) = components.split_first() {
            self.directories
                .entry(first.clone())
                .or_insert_with(|| Node {
                    stats: Some((0, 0)),
                    ..Default::default()
                })
                .insert(entry, rest);
        } else {
            self.files.push(entry);
        }
    }
}
pub(super) fn directories(entries: &[Entry]) -> HashSet<Directory> {
    entries
        .iter()
        .flat_map(|entry| {
            entry
                .file
                .path()
                .ancestors()
                .skip(1)
                .filter(|p| !p.as_os_str().is_empty())
                .map(|path| Directory {
                    group: Group::of(entry),
                    path: path.into(),
                })
        })
        .collect()
}
fn row(entry: &Entry) -> Row {
    let file = &entry.file;
    Row {
        path: entry.id(),
        label: file
            .previous_path()
            .map(|p| format!("{} → {}", p.display(), file.path().display()))
            .unwrap_or_else(|| file.path().display().to_string()),
        status: Some(if entry.source == ChangeSource::Untracked {
            "diff-untracked"
        } else {
            kind_key(file.kind())
        }),
        stats: file.stats(),
        failed: false,
    }
}
pub(super) fn build(
    entries: &[Entry],
    view: GitView,
    closed_groups: &HashSet<Group>,
    closed: &HashSet<Directory>,
    query: &str,
) -> (Vec<Item>, Vec<String>) {
    let mut items = Vec::new();
    let mut order = Vec::new();
    let mut conflicts = HashSet::new();
    for group in [Group::Conflicts, Group::Staged, Group::Unstaged] {
        let matching: Vec<_> = entries
            .iter()
            .filter(|entry| {
                Group::of(entry) == group
                    && (entry
                        .file
                        .path()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(query)
                        || entry
                            .file
                            .previous_path()
                            .is_some_and(|p| p.to_string_lossy().to_lowercase().contains(query)))
            })
            .filter(|entry| {
                group != Group::Conflicts || conflicts.insert(entry.file.path().to_owned())
            })
            .collect();
        if group == Group::Conflicts && matching.is_empty() {
            continue;
        }
        let visible = !closed_groups.contains(&group);
        items.push(Item::Group(group, matching.len(), !visible));
        if view == GitView::List {
            for entry in matching {
                order.push(entry.id());
                if visible {
                    items.push(Item::File(row(entry), status_code(entry), 0));
                }
            }
        } else {
            let mut root = Node {
                stats: Some((0, 0)),
                ..Default::default()
            };
            for entry in matching {
                let components: Vec<_> = entry
                    .file
                    .path()
                    .parent()
                    .unwrap_or(Path::new(""))
                    .components()
                    .map(|c| c.as_os_str().to_owned())
                    .collect();
                root.insert(entry, &components);
            }
            flatten(
                &root,
                group,
                Path::new(""),
                closed,
                !query.is_empty(),
                visible,
                &mut items,
                &mut order,
            );
        }
    }
    (items, order)
}
#[allow(clippy::too_many_arguments)]
fn flatten(
    node: &Node<'_>,
    group: Group,
    path: &Path,
    closed: &HashSet<Directory>,
    searching: bool,
    visible: bool,
    items: &mut Vec<Item>,
    order: &mut Vec<String>,
) {
    for (name, child) in &node.directories {
        let path = path.join(name);
        let key = Directory {
            group,
            path: path.clone(),
        };
        let collapsed = !searching && closed.contains(&key);
        if visible {
            items.push(Item::Directory(key, child.count, collapsed, child.stats));
        }
        flatten(
            child,
            group,
            &path,
            closed,
            searching,
            visible && !collapsed,
            items,
            order,
        );
    }
    let mut files = node.files.clone();
    files.sort_by(|a, b| {
        a.file
            .path()
            .cmp(b.file.path())
            .then_with(|| source_order(a.source).cmp(&source_order(b.source)))
    });
    for entry in files {
        order.push(entry.id());
        if visible {
            items.push(Item::File(
                row(entry),
                status_code(entry),
                path.components().count() + 1,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn raw_path_tree_orders_directories_before_files_and_keeps_source_identities() {
        let entries: Vec<_> = [
            (ChangeSource::Staged, "src/a.rs", ChangeKind::Modified),
            (ChangeSource::Unstaged, "src/a.rs", ChangeKind::Modified),
            (ChangeSource::Unstaged, "src", ChangeKind::Deleted),
            (
                ChangeSource::Unstaged,
                "src/deep/gone.rs",
                ChangeKind::Deleted,
            ),
            (ChangeSource::Untracked, "notes.md", ChangeKind::Added),
            (ChangeSource::Unstaged, "new/place.rs", ChangeKind::Renamed),
        ]
        .into_iter()
        .map(|(source, path, kind)| Entry {
            source,
            file: Change::for_test(path.into(), kind),
        })
        .collect();
        let (items, order) = build(
            &entries,
            GitView::Tree,
            &HashSet::new(),
            &HashSet::new(),
            "",
        );
        assert_eq!(order, [0, 5, 3, 1, 4, 2].map(|i| entries[i].id()));
        let dirs: Vec<_> = items
            .iter()
            .filter_map(|i| match i {
                Item::Directory(key, count, _, stats) => Some((key, *count, *stats)),
                _ => None,
            })
            .collect();
        let staged = dirs
            .iter()
            .find(|(key, _, _)| key.group == Group::Staged)
            .unwrap();
        let unstaged = dirs
            .iter()
            .find(|(key, _, _)| key.group == Group::Unstaged && key.path == Path::new("src"))
            .unwrap();
        assert_ne!(staged.0.id(), unstaged.0.id());
        assert_eq!(unstaged.1, 2);
        assert_eq!(unstaged.2, Some((2, 2)));
        assert!(!items.iter().any(|item| matches!(item, Item::File(row,_,_) if item.id() == unstaged.0.id() && row.label == "src")));
    }

    #[test]
    fn incomplete_directory_stats_never_become_a_complete_total() {
        let entry = Entry {
            source: ChangeSource::Unstaged,
            file: Change::for_test("src/a.rs".into(), ChangeKind::Modified),
        };
        let mut unknown = Node::default();
        unknown.insert(&entry, &[]);
        assert_eq!(unknown.count, 1);
        assert_eq!(unknown.stats, None);
        let mut complete = Node {
            stats: Some((0, 0)),
            ..Default::default()
        };
        complete.insert(&entry, &[]);
        assert_eq!(complete.stats, Some((1, 1)));
    }

    #[test]
    fn search_temporarily_expands_without_mutating_closed_directories() {
        let entries = vec![Entry {
            source: ChangeSource::Unstaged,
            file: Change::for_test("a/b/deleted.rs".into(), ChangeKind::Deleted),
        }];
        let closed = directories(&entries);
        let (_, full_order) = build(&entries, GitView::Tree, &HashSet::new(), &closed, "");
        let (collapsed, _) = build(&entries, GitView::Tree, &HashSet::new(), &closed, "");
        assert!(!collapsed.iter().any(|i| matches!(i, Item::File(..))));
        let (search, order) = build(&entries, GitView::Tree, &HashSet::new(), &closed, "deleted");
        assert_eq!(order, full_order);
        assert_eq!(
            search
                .iter()
                .filter(|i| matches!(i, Item::File(..)))
                .count(),
            1
        );
        assert!(
            search
                .iter()
                .all(|i| !matches!(i, Item::Directory(_, _, true, _)))
        );
        assert_eq!(closed.len(), 2);
        let (restored, _) = build(&entries, GitView::Tree, &HashSet::new(), &closed, "");
        assert!(!restored.iter().any(|i| matches!(i, Item::File(..))));
    }
}
