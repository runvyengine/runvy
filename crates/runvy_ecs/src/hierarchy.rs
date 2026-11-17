use crate::Entity;

#[derive(Debug, Default, Clone, Eq, PartialEq, Copy)]
pub struct ChildOf(Entity);

impl ChildOf {
    pub(crate) fn new(parent: Entity) -> Self {
        Self(parent)
    }

    pub fn parent(self) -> Entity {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct Children(Vec<Entity>);

impl Children {
    pub(crate) fn push(&mut self, child: Entity) {
        debug_assert!(!self.0.contains(&child));
        self.0.push(child);
    }

    pub(crate) fn remove(&mut self, child: Entity) {
        self.0.retain(|&e| e != child);
    }

    pub(crate) fn snapshot(&self) -> Vec<Entity> {
        self.0.clone()
    }

    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.0.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[Entity] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HierarchyError {
    EntityNotFound(Entity),
    SelfParent,
    Cycle,
}
