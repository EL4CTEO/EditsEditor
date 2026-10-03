//! Snapshot-based undo/redo. Project documents are small, so whole-document snapshots are cheap,
//! simple and bulletproof (no inverse-operation bugs).

use std::time::SystemTime;

use crate::model::Project;

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub label: String,
    pub at: SystemTime,
    snapshot: Project,
}

#[derive(Clone, Debug)]
pub struct History {
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    limit: usize,
    /// Monotonic revision number, increments on every change (used by watchers / viewers).
    pub revision: u64,
}

impl Default for History {
    fn default() -> Self {
        History::new(256)
    }
}

impl History {
    pub fn new(limit: usize) -> Self {
        History { undo: vec![], redo: vec![], limit, revision: 0 }
    }

    /// Record the state *before* a change.
    pub fn record(&mut self, label: impl Into<String>, before: &Project) {
        self.undo.push(HistoryEntry { label: label.into(), at: SystemTime::now(), snapshot: before.clone() });
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision += 1;
    }

    /// Undo: returns the label of the undone change.
    pub fn undo(&mut self, current: &mut Project) -> Option<String> {
        let e = self.undo.pop()?;
        let label = e.label.clone();
        let now = std::mem::replace(current, e.snapshot);
        self.redo.push(HistoryEntry { label: e.label, at: SystemTime::now(), snapshot: now });
        self.revision += 1;
        Some(label)
    }

    pub fn redo(&mut self, current: &mut Project) -> Option<String> {
        let e = self.redo.pop()?;
        let label = e.label.clone();
        let now = std::mem::replace(current, e.snapshot);
        self.undo.push(HistoryEntry { label: e.label, at: SystemTime::now(), snapshot: now });
        self.revision += 1;
        Some(label)
    }

    pub fn labels(&self) -> (Vec<&str>, Vec<&str>) {
        (self.undo.iter().map(|e| e.label.as_str()).collect(), self.redo.iter().rev().map(|e| e.label.as_str()).collect())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo() {
        let mut p = Project::new("a", 10, 10, 24.0, 1.0);
        let mut h = History::default();
        h.record("rename", &p);
        p.meta.name = "b".into();
        assert_eq!(h.undo(&mut p).as_deref(), Some("rename"));
        assert_eq!(p.meta.name, "a");
        h.redo(&mut p);
        assert_eq!(p.meta.name, "b");
    }
}
