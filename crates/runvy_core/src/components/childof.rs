use runvy_ecs::Entity;
use runvy_macros::Scriptable;

#[derive(Clone, Copy, Debug, Default, Scriptable)]
#[script(crate = "::runvy_script_api", builtin)]
pub struct ChildOf {
    pub parent: Entity,
}

impl ChildOf {
    pub fn new(parent: Entity) -> Self {
        Self { parent }
    }
}
