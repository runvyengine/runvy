use runvy_engine::{
    asset::load_image,
    core::{
        components::{Tilemap, TilemapLayer, TilemapRenderer, Transform},
        glam::USizeVec2,
    },
    ecs::World,
};

pub fn spawn_tilemap(world: &mut World) -> u64 {
    let mut tm = Tilemap::builder(100, 100)
        .centered()
        .tile_size(USizeVec2::new(32, 32))
        .atlas(
            load_image!("assets/TilemapTest.png"),
            Some("assets/TilemapTest.png".to_string()),
            1,
            1,
        )
        .build();
    let mut tml = TilemapLayer::new("test".into(), 100, 100);
    for i in 0..100 {
        for j in 0..100 {
            tml.set(i, j, tm.id_for_frame(0));
        }
    }
    tm.add_layer(tml);

    world.spawn((Transform::default(), tm, TilemapRenderer::new()))
}
