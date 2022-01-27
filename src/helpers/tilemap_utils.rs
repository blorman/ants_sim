use bevy::prelude::*;
use bevy::sprite::collide_aabb::{collide, Collision};
use bevy_ecs_tilemap::{ChunkPos, MapQuery, TilePos};

pub fn world_pos_from_tile_pos(
    tile_pos: TilePos,
    map_query: &mut MapQuery,
    map_transform: &Transform,
    map_id: u16,
    layer_id: u16,
) -> Vec3 {
    if let Some((_entity, layer)) = map_query.get_layer(map_id, layer_id) {
        let grid_size = layer.settings.grid_size.extend(0.0);
        return Vec3::new(tile_pos.0 as f32, tile_pos.1 as f32, 0.0) * grid_size
            + map_transform.translation
            + grid_size / 2.0;
    }
    Vec3::new(0.0, 0.0, 0.0)
}

pub fn tile_pos_from_world_pos(
    world_pos: &Vec3,
    map_query: &mut MapQuery,
    map_transform: &Transform,
    map_id: u16,
    layer_id: u16,
) -> TilePos {
    if let Some((_entity, layer)) = map_query.get_layer(map_id, layer_id) {
        let grid_size = layer.settings.grid_size;
        let tile_pos_vec3 = (*world_pos - map_transform.translation).truncate() / grid_size;
        return TilePos(tile_pos_vec3.x as u32, tile_pos_vec3.y as u32);
    }
    TilePos(0, 0)
}

pub fn collide_tiles_with_rect(
    pos: Vec3,
    dimensions: Vec2,
    map_query: &mut MapQuery,
    map_transform: &Transform,
    map_id: u16,
    layer_id: u16,
) -> Vec<Collision> {
    let mut collisions = Vec::new();
    let box_bottom_left = pos - dimensions.extend(0.0) / 2.0;
    let box_top_right = pos + dimensions.extend(0.0) / 2.0;
    let tile_pos_bottom_left =
        tile_pos_from_world_pos(&box_bottom_left, map_query, map_transform, map_id, layer_id);
    let tile_pos_top_right =
        tile_pos_from_world_pos(&box_top_right, map_query, map_transform, map_id, layer_id);
    if let Some((_entity, layer)) = map_query.get_layer(map_id, layer_id) {
        let grid_size = layer.settings.grid_size;
        for i in tile_pos_bottom_left.0..=tile_pos_top_right.0 {
            for j in tile_pos_bottom_left.1..=tile_pos_top_right.1 {
                if let Ok(_tile_entity) = map_query.get_tile_entity(TilePos(i, j), map_id, layer_id)
                {
                    let tile_world_pos = world_pos_from_tile_pos(
                        TilePos(i, j),
                        map_query,
                        map_transform,
                        map_id,
                        layer_id,
                    );
                    let collision = collide(pos, dimensions, tile_world_pos, grid_size);
                    if let Some(collision) = collision {
                        collisions.push(collision);
                    }
                }
            }
        }
    }
    return collisions;
}

// pub fn collide_tiles_with_sector(
//     pos: Vec3,
//     radius: f32,
//     angle1: f32,
//     angle2: f32,
//     map_query: &mut MapQuery,
//     map_transform: &Transform,
//     map_id: u16,
//     layer_id: u16,
// ) -> Vec<Collision> {
//     // https://stackoverflow.com/questions/13652518/efficiently-find-points-inside-a-circle-sector
//     let mut collisions = Vec::new();
//     return collisions;
// }

pub fn despawn_layer_tiles_and_notify_chunks(
    commands: &mut Commands,
    map_query: &mut MapQuery,
    map_id: u16,
    layer_id: u16,
) {
    map_query.despawn_layer_tiles(commands, map_id, layer_id);
    let mut chunk_entities = Vec::new();
    if let Some((_entity, layer)) = map_query.get_layer(map_id, layer_id) {
        for i in 0..layer.settings.map_size.0 {
            for j in 0..layer.settings.map_size.1 {
                if let Some(chunk_entity) = layer.get_chunk(ChunkPos(i, j)) {
                    chunk_entities.push(chunk_entity);
                }
            }
        }
    }
    for chunk_entity in chunk_entities {
        map_query.notify_chunk(chunk_entity);
    }
}

// #[cfg(test)]
// mod tests {
//     use bevy::prelude::*;
//     use bevy_ecs_tilemap::MapQuery;
//     use bevy_ecs_tilemap::TilemapPlugin;

//     #[test]
//     fn it_works() {
//         let mut world = World::new();
//         let mut app = App::from_world(&mut world);
//         app.add_plugin(TilemapPlugin);
//         // let mut schedule = Schedule::default();
//         // let mut test_stage = SystemStage::parallel();
//         // test_stage.add_plugin(TilemapPlugin);
//         // schedule.add_stage("test_stage", test_stage);
//         for iteration in 1..=10 {
//             println!("Simulating frame {}/10", iteration);
//             app.update();
//         }

//         // https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/examples/resources.rs
//         // https://github.com/bevyengine/bevy/issues/2687
//         // let mut state: SystemState<(Commands, ResMut<ClientList>, Query<&mut ActorInput>)> = SystemState::new(&mut world);
//         // let (commands, client_list, actor_input) = state.get_mut(&mut World);

//         //     let map_query = MapQuery{};
//         //     let map_transform = Tranform{};
//         //     let v = world_pos_from_tile_pos(
//         // TilePos(0,0),
//         // map_query,
//         // map_transform,
//         // 0,
//         // 0,
//     }

//     // #[test]
//     // fn it_works() {
//     //     assert_eq!(2 + 2, 4);
//     // }
// }
