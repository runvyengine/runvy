#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializedTypeKind {
    Component,
    Script,
}

#[derive(Debug, Clone)]
pub struct SerializedField {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub struct SerializedTypeEntry {
    pub type_name: String,
    pub kind: SerializedTypeKind,
    pub fields: Vec<SerializedField>,
}

#[derive(Debug, Clone, Default)]
pub struct SerializedTypeStorage {
    pub entries: Vec<SerializedTypeEntry>,
}

impl SerializedTypeStorage {
    pub fn entries_of_kind(
        &self,
        kind: SerializedTypeKind,
    ) -> impl Iterator<Item = &SerializedTypeEntry> {
        self.entries.iter().filter(move |entry| entry.kind == kind)
    }

    pub fn remove(&mut self, kind: SerializedTypeKind, type_name: &str) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|entry| !(entry.kind == kind && entry.type_name == type_name));
        before != self.entries.len()
    }

    pub fn upsert(&mut self, entry: SerializedTypeEntry) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|existing| existing.kind == entry.kind && existing.type_name == entry.type_name)
        {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }
}
