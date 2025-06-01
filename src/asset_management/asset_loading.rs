use avian3d::prelude::{Collider, ColliderConstructor, RigidBody};
use bevy::{asset::LoadState, pbr::ExtendedMaterial, prelude::*};

use crate::{
    GameState,
    rendering::{
        section_color_prepass::{ATTRIBUTE_SECTION_COLOR, DrawSection},
        unlit_material::{UnlitMaterial, UnlitMaterialExtension},
    },
};

use super::asset_tag_components::{NeedsRigidBody, RoomWall};

pub(crate) fn assets_plugin(app: &mut App) {
    app.init_state::<AssetLoaderState>()
        .init_resource::<GameAssets>()
        .add_systems(
            Update,
            (
                check_asset_loading.run_if(in_state(AssetLoaderState::Loading)),
                add_rigidbodies_to_colliders,
            ),
        )
        .add_systems(OnEnter(AssetLoaderState::Loading), on_start_loading)
        .add_systems(OnEnter(AssetLoaderState::Postprocess), postprocess_assets);
}

#[derive(SubStates, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[source(GameState = GameState::Loading)]
pub enum AssetLoaderState {
    #[default]
    Loading,
    Postprocess,
}

#[derive(Resource, Default)]
pub struct GameAssets {
    pub main_menu_environment: Handle<Scene>,
    pub weighted_cube_cyan: Handle<Scene>,
    pub font: Handle<Font>,
}

#[derive(Component)]
pub struct LoadingAsset(pub UntypedHandle);

fn on_start_loading(
    mut commands: Commands,
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
) {
    game_assets.main_menu_environment =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("scenes/jam6scene1.glb"));
    commands.spawn(LoadingAsset(
        game_assets.main_menu_environment.clone().into(),
    ));

    game_assets.weighted_cube_cyan =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("scenes/weighted_Cube_cyan.glb"));
    commands.spawn(LoadingAsset(game_assets.weighted_cube_cyan.clone().into()));

    game_assets.font = asset_server.load("fonts/Ronysiswadi15-51Dv8.ttf");
    commands.spawn(LoadingAsset(game_assets.font.clone().into()));

    commands.set_state(AssetLoaderState::Loading);
}

fn check_asset_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loading_assets: Query<(Entity, &LoadingAsset)>,
) {
    let all_loaded = &loading_assets.iter().all(|(_, loading_asset)| {
        matches!(
            asset_server.get_load_state(&loading_asset.0),
            Some(LoadState::Loaded)
        )
    });

    if *all_loaded {
        info!("All assets loaded successfully");
        commands.set_state(AssetLoaderState::Postprocess);
        loading_assets.iter().for_each(|(entity, _)| {
            commands.entity(entity).despawn();
        });
    }
}

fn postprocess_assets(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut scenes: ResMut<Assets<Scene>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // set up materials and colliders for everything
    let scenes_to_process = vec![
        game_assets.main_menu_environment.clone(),
        game_assets.weighted_cube_cyan.clone(),
    ];

    for scene_handle in scenes_to_process {
        if let Some(scene) = scenes.get_mut(&scene_handle) {
            let mut materials_to_process = Vec::new();
            for entity_ref in scene.world.iter_entities() {
                if let Some(material_handle) = scene
                    .world
                    .get::<MeshMaterial3d<StandardMaterial>>(entity_ref.id())
                {
                    materials_to_process.push((entity_ref.id(), material_handle.0.clone()));
                }
            }

            for (entity, material_handle) in materials_to_process {
                if let Some(old_material) = standard_materials.get_mut(&material_handle) {
                    //old_material.reflectance = 0.0;

                    let default_new_material = ExtendedMaterial {
                        base: old_material.clone(),
                        extension: UnlitMaterialExtension { foo: 0.0 },
                    };

                    // Example of singling out a specific marked object to modify the material
                    // // marker components are on the mesh parent
                    // let new_material = if let Some(child_of) = scene.world.entity(entity).get::<ChildOf>() {
                    //     if scene.world.entity(child_of.0).contains::<RoomWalls>() {
                    //         let mut new_old_material = old_material.clone();
                    //         new_old_material.cull_mode = None;

                    //         ExtendedMaterial {
                    //             base: new_old_material,
                    //             extension: UnlitMaterialExtension { foo: 0.0 },
                    //         }
                    //     } else {
                    //         default_new_material
                    //     }
                    // } else {
                    //     default_new_material
                    // };

                    scene
                        .world
                        .entity_mut(entity)
                        .remove::<MeshMaterial3d<StandardMaterial>>()
                        .insert(MeshMaterial3d(unlit_materials.add(default_new_material)));
                }
            }

            // Do any mesh postprocessing we need
            let mut entities_to_process = Vec::new();
            for entity_ref in scene.world.iter_entities() {
                let entity = entity_ref.id();
                if let Some(mesh_handle) = scene.world.get::<Mesh3d>(entity) {
                    entities_to_process.push((entity, mesh_handle.clone()));
                }
            }

            for (entity, mesh_handle) in entities_to_process.iter() {
                if let Some(mesh) = meshes.get_mut(&*mesh_handle) {
                    // convert vertex colors to the section color our outline effect expects
                    // TODO: Should we remove the vertex color attribute afterwards?
                    if let Some(vertex_colors) = mesh.attribute(Mesh::ATTRIBUTE_COLOR).cloned() {
                        mesh.insert_attribute(ATTRIBUTE_SECTION_COLOR, vertex_colors);

                        // Configure entities with the attribute to be drawn with section outlines
                        scene.world.entity_mut(*entity).insert(DrawSection);
                    } else {
                        warn!(
                            "Mesh on entity {:?} doesn't have vertex colors to convert",
                            entity
                        );
                    }

                    scene
                        .world
                        .entity_mut(*entity)
                        .insert(ColliderConstructor::TrimeshFromMesh);
                }
            }

            for (_, mesh_handle) in entities_to_process {
                if let Some(mesh) = meshes.get_mut(&mesh_handle) {
                    if let Some(_) = mesh.attribute(Mesh::ATTRIBUTE_COLOR).cloned() {
                        mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
                    }
                }
            }
        }
    }

    // set up static environments
    let environments_to_process = vec![game_assets.main_menu_environment.clone()];

    for scene_handle in environments_to_process {
        // Find all entities with colliders and assign NeedsRigidBody with RigidBody::Static
        if let Some(scene) = scenes.get_mut(&scene_handle) {
            let mut entities_with_colliders = Vec::new();
            for entity_ref in scene.world.iter_entities() {
                let entity = entity_ref.id();
                if scene.world.get::<ColliderConstructor>(entity).is_some() {
                    entities_with_colliders.push(entity);
                }
            }

            for entity in entities_with_colliders {
                scene.world.entity_mut(entity).insert(NeedsRigidBody {
                    kind: RigidBody::Static,
                });
            }
        }
    }

    commands.spawn(SceneRoot(game_assets.main_menu_environment.clone()));
    //commands.set_state(GameState::MainMenu);
    commands.set_state(GameState::Playing);
}


fn add_rigidbodies_to_colliders(
    mut commands: Commands,
    q_colliders_without_rigidbody: Query<(Entity, &NeedsRigidBody)>,
) {
    for (entity, nrb) in &q_colliders_without_rigidbody {
        commands.entity(entity).insert(nrb.kind);
    }
}
