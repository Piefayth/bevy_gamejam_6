use std::net::Ipv4Addr;

use bevy::{
    ecs::reflect::ReflectCommandExt, prelude::*, reflect::serde::ReflectDeserializer
};

#[cfg(not(target_arch = "wasm32"))]
use bevy::remote::{http::RemoteHttpPlugin, RemotePlugin};

use serde::{de::DeserializeSeed};
use serde_json::Value;

pub struct UnityPlugin {
    brp: bool,
}

impl Default for UnityPlugin {
    fn default() -> Self {
        Self { brp: true }
    }
}

impl Plugin for UnityPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.brp {
            app.add_plugins((
                RemotePlugin::default(),
                RemoteHttpPlugin::default()
                    .with_address(Ipv4Addr::LOCALHOST)
                    .with_port(5309),
            ));
        }

        app.add_observer(apply_bevity_components);
    }
}

fn apply_bevity_components(
    trigger: Trigger<
        OnAdd,
        (
            GltfExtras,
        ),
    >,
    type_registry: Res<AppTypeRegistry>,
    gltf_extras: Query<&GltfExtras>,
    names: Query<&Name>,
    mut commands: Commands,
) {
    let entity = trigger.target();
    let gltf_extra = gltf_extras.get(entity).map(|v| &v.value);
    for extras in [
        gltf_extra,
    ]
    .iter()
    .filter_map(|p| p.ok())
    {
        let obj = match serde_json::from_str(extras) {
            Ok(Value::Object(obj)) => obj,
            Ok(Value::Null) => {
                if let Ok(name) = names.get(entity) {
                    trace!(
                        "entity {:?} with name {name} had gltf extras which could not be parsed as a serde_json::Value::Object; parsed as Null",
                        entity
                    );
                } else {
                    trace!(
                        "entity {:?} with no Name had gltf extras which could not be parsed as a serde_json::Value::Object; parsed as Null",
                        entity
                    );
                }
                continue;
            }
            Ok(value) => {
                let name = names.get(entity).ok();
                trace!(?entity, ?name, parsed_as=?value, "gltf extras which could not be parsed as a serde_json::Value::Object");
                continue;
            }
            Err(err) => {
                let name = names.get(entity).ok();
                trace!(
                    ?entity,
                    ?name,
                    ?err,
                    "gltf extras which could not be parsed as a serde_json::Value::Object"
                );
                continue;
            }
        };

        let bevity = match obj.get("bevity") {
            Some(Value::Array(components)) => components,
            _ => continue
        };

        for json_component in bevity.iter() {
            let type_registry = type_registry.read();

            let reflect_deserializer =
                ReflectDeserializer::new(&type_registry);
            let reflect_value = match reflect_deserializer
                .deserialize(json_component)
            {
                Ok(value) => value,
                Err(err) => {
                    error!(
                        ?err,
                        ?obj,
                        "failed to instantiate component data from glTF data"
                    );
                    continue;
                }
            };

            commands
                .entity(entity)
                .insert_reflect(reflect_value);
        }
    }
}
