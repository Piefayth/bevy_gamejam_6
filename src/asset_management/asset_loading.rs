use avian3d::prelude::{Collider, RigidBody};
use bevy::{asset::LoadState, color::palettes::css::WHITE, pbr::ExtendedMaterial, prelude::*};

use crate::{
    asset_management::asset_tag_components::{FancyMesh, WeightedCube},
    rendering::{
        section_color_prepass::{DrawSection, ATTRIBUTE_SECTION_COLOR},
        unlit_material::{UnlitMaterial, UnlitMaterialExtension, UnlitParams},
    },
    GameState,
};

use super::asset_tag_components::{
    CubeSpitter, Door, DoorPole, Inert, NeedsRigidBody, PowerButton, SignalSpitter,
    StandingCubeSpitter,
};

pub(crate) fn assets_plugin(app: &mut App) {
    app.init_state::<AssetLoaderState>()
        .init_resource::<GameAssets>()
        .add_systems(
            Update,
            (
                check_asset_loading.run_if(in_state(AssetLoaderState::Loading)),
                (assign_colliders_to_meshes,
                add_rigidbodies_to_colliders,
                ).chain()
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
    // scenes
    pub main_menu_environment: Handle<Scene>,

    // objects, in scene form
    pub weighted_cube_cyan: Handle<Scene>,

    // meshes

    // materials
    pub cyan_signal_material: Handle<UnlitMaterial>,

    // audio

    // fonts
    pub font: Handle<Font>,
}

#[derive(Component)]
pub struct LoadingAsset(pub UntypedHandle);

fn on_start_loading(
    mut commands: Commands,
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
    mut unlit_materials: ResMut<Assets<UnlitMaterial>>,
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

    game_assets.cyan_signal_material = unlit_materials.add(UnlitMaterial {
        base: StandardMaterial {
            base_color: LinearRgba::new(4. / 255., 149. / 255., 249. / 255., 1.0).into(),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        },
        extension: UnlitMaterialExtension {
            params: UnlitParams {
                intensity: 1.0,
                alpha: 0.75,
                blend_color: WHITE.into(),
                blend_factor: 0.0,
                ..default()
            },
        },
    });

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
                        extension: UnlitMaterialExtension {
                            params: UnlitParams {
                                intensity: 1.0,
                                alpha: 1.0,
                                blend_color: WHITE.into(),
                                blend_factor: 0.0,
                                grey_threshold: 0.2,
                            },
                        },
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
                if let Some(mesh) = meshes.get_mut(mesh_handle) {
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
                        .insert(NeedsRigidBody {
                    kind: RigidBody::Static,
                });
                }
            }

            for (_, mesh_handle) in entities_to_process {
                if let Some(mesh) = meshes.get_mut(&mesh_handle) {
                    if mesh.attribute(Mesh::ATTRIBUTE_COLOR).cloned().is_some() {
                        mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
                    }
                }
            }
        }
    }

    // // set up static environments
    // let environments_to_process = vec![game_assets.main_menu_environment.clone()];

    // for scene_handle in environments_to_process {
    //     // Find all entities with colliders and assign NeedsRigidBody with RigidBody::Static
    //     if let Some(scene) = scenes.get_mut(&scene_handle) {
    //         let mut entities_with_colliders = Vec::new();
    //         for entity_ref in scene.world.iter_entities() {
    //             let entity = entity_ref.id();
    //             if scene.world.get::<ColliderConstructor>(entity).is_some() {
    //                 entities_with_colliders.push(entity);
    //             }
    //         }

    //         for entity in entities_with_colliders {
    //             scene.world.entity_mut(entity).insert(NeedsRigidBody {
    //                 kind: RigidBody::Static,
    //             });
    //         }
    //     }
    // }

    commands.spawn(SceneRoot(game_assets.main_menu_environment.clone()));
    //commands.set_state(GameState::MainMenu);
    commands.set_state(GameState::Playing);
}


fn assign_colliders_to_meshes(
    mut commands: Commands,
    // Query for mesh entities that don't have colliders yet
    mesh_entities: Query<
        (Entity, &Mesh3d, Option<&ChildOf>),
        (Without<Collider>, Added<Mesh3d>)
    >,
    // Query for entities that should use trimesh colliders
    trimesh_entities: Query<(), Or<(With<Door>, With<FancyMesh>)>>,
    // Query for entities with WeightedCube component
    weighted_cube_entities: Query<(), With<WeightedCube>>,
    // Query for parent relationships
    parent_query: Query<&ChildOf>,
    meshes: Res<Assets<Mesh>>,
) {
    for (entity, mesh_handle, parent) in &mesh_entities {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            // Check if entity itself has components that should use TrimeshFromMesh
            let entity_needs_trimesh = trimesh_entities.contains(entity);
           
            // Check if parent has components that should use TrimeshFromMesh
            let parent_needs_trimesh = if let Some(parent) = parent {
                trimesh_entities.contains(parent.parent())
            } else {
                false
            };

            // ok we dont need this but im too scared to break anything sooo
            let has_weighted_cube_parent = check_for_weighted_cube_in_hierarchy(
                entity, 
                &weighted_cube_entities, 
                &parent_query
            );

            let collider = if entity_needs_trimesh || parent_needs_trimesh {
                Collider::trimesh_from_mesh(mesh)
            } else {
                Collider::convex_hull_from_mesh(mesh)
            };

            if let Some(collider) = collider {
                let mut entity_commands = commands.entity(entity);
                entity_commands.insert(collider);

                // Only add RigidBody if no WeightedCube parent exists
                if !has_weighted_cube_parent {
                    entity_commands.insert(NeedsRigidBody {
                        kind: RigidBody::Static,
                    });
                }
            } else {
                warn!("Failed to create collider for mesh on entity {:?}", entity);
            }
        }
    }
}

// ok we dont need this but im too scared to break anything sooo
fn check_for_weighted_cube_in_hierarchy(
    mut current_entity: Entity,
    weighted_cube_entities: &Query<(), With<WeightedCube>>,
    parent_query: &Query<&ChildOf>,
) -> bool {
    // First check the entity itself
    if weighted_cube_entities.contains(current_entity) {
        return true;
    }

    // Then traverse up the parent hierarchy
    while let Ok(child_of) = parent_query.get(current_entity) {
        let parent_entity = child_of.parent();
        if weighted_cube_entities.contains(parent_entity) {
            return true;
        }
        current_entity = parent_entity;
    }

    false
}
fn add_rigidbodies_to_colliders(
    mut commands: Commands,
    q_colliders_without_rigidbody: Query<(Entity, &NeedsRigidBody, &ChildOf)>,
    q_exclusions: Query<
        (),
        Or<(
            With<SignalSpitter>,
            With<CubeSpitter>,
            With<DoorPole>,
            With<Door>,
            With<Inert>,
            With<StandingCubeSpitter>,
            With<PowerButton>,
            With<WeightedCube>,
        )>,
    >, // we will add these RBs later during registration
    parent_query: Query<&ChildOf>,
) {
    for (entity, nrb, child_of) in &q_colliders_without_rigidbody {
        // Check if any parent in the hierarchy has exclusion components
        let mut has_excluded_parent = false;
        let mut current_parent = child_of.parent();
        
        loop {
            if q_exclusions.contains(current_parent) {
                has_excluded_parent = true;
                break;
            }
            
            if let Ok(parent_child_of) = parent_query.get(current_parent) {
                current_parent = parent_child_of.parent();
            } else {
                break; // No more parents
            }
        }
        
        if !has_excluded_parent {
            commands
                .entity(entity)
                .insert(nrb.kind)
                .remove::<NeedsRigidBody>();
        } else {
            commands.entity(entity).remove::<NeedsRigidBody>();
        }
    }
}
