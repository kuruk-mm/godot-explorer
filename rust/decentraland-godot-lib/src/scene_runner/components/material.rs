use std::collections::HashSet;

use crate::{
    dcl::{
        components::{
            material::{DclMaterial, DclSourceTex, DclTexture},
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::{MaterialItem, Scene},
};
use godot::{
    engine::{ImageTexture, MeshInstance3D, StandardMaterial3D},
    prelude::{utilities::weakref, *},
};

pub fn update_material(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let material_component = SceneCrdtStateProtoComponents::get_material(crdt_state);
    let mut content_manager = godot_dcl_scene
        .root_node
        .get_node("/root/content_manager".into())
        .unwrap()
        .share();
    let content_mapping_files = scene
        .content_mapping
        .get("content")
        .unwrap()
        .to::<Dictionary>();

    if let Some(material_dirty) = dirty_lww_components.get(&SceneComponentId::MATERIAL) {
        for entity in material_dirty {
            let new_value = material_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let dcl_material = if let Some(material) = new_value.value.as_ref() {
                material
                    .material
                    .as_ref()
                    .map(|material| DclMaterial::from_proto(material, &content_mapping_files))
            } else {
                None
            };

            let node = godot_dcl_scene.ensure_node_mut(entity);

            if let Some(dcl_material) = dcl_material {
                let previous_dcl_material = node.material.as_ref();
                if let Some(previous_dcl_material) = previous_dcl_material {
                    if previous_dcl_material.eq(&dcl_material) {
                        continue;
                    }
                }

                let existing_material =
                    if let Some(material_item) = scene.materials.get(&dcl_material) {
                        let material_item = material_item.weak_ref.call("get_ref", &[]);

                        if material_item.is_nil() {
                            None
                        } else {
                            Some(material_item)
                        }
                    } else {
                        None
                    };

                if existing_material.is_none() {
                    for tex in dcl_material.get_textures().into_iter().flatten() {
                        if let DclSourceTex::Texture(hash) = &tex.source {
                            content_manager.call(
                                "fetch_texture_by_hash".into(),
                                &[
                                    GodotString::from(hash).to_variant(),
                                    scene.content_mapping.to_variant(),
                                ],
                            );
                        }
                    }
                }

                let mut godot_material = if let Some(material) = existing_material {
                    material.to::<Gd<StandardMaterial3D>>()
                } else {
                    let godot_material = StandardMaterial3D::new();

                    let waiting_textures = {
                        match &dcl_material {
                            DclMaterial::Unlit(unlit) => unlit.texture.is_some(),
                            DclMaterial::Pbr(pbr) => {
                                pbr.texture.is_some()
                                    || pbr.bump_texture.is_some()
                                    || pbr.alpha_texture.is_some()
                                    || pbr.emissive_texture.is_some()
                            }
                        }
                    };

                    scene.materials.insert(
                        dcl_material.clone(),
                        MaterialItem {
                            weak_ref: weakref(godot_material.to_variant()),
                            waiting_textures,
                            alive: true,
                        },
                    );
                    godot_material
                };

                match &dcl_material {
                    DclMaterial::Unlit(unlit) => {
                        godot_material.set_metallic(0.0);
                        godot_material.set_roughness(0.0);
                        godot_material.set_specular(0.0);
                        godot_material.set_albedo(unlit.diffuse_color.0.to_godot());
                    }
                    DclMaterial::Pbr(pbr) => {
                        godot_material.set_metallic(pbr.metallic.0);
                        godot_material.set_roughness(pbr.roughness.0);
                        godot_material.set_specular(pbr.specular_intensity.0);

                        let emission = pbr
                            .emissive_color
                            .0
                            .clone()
                            .multiply(pbr.emissive_intensity.0);
                        godot_material.set_emission(emission.to_godot());
                        godot_material.set_albedo(pbr.albedo_color.0.to_godot());
                    }
                }

                let mesh_renderer = node
                    .base
                    .try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

                if let Some(mut mesh_renderer) = mesh_renderer {
                    mesh_renderer.set_surface_override_material(0, godot_material.upcast());
                }
            } else {
                let mesh_renderer = node
                    .base
                    .try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

                if let Some(mut mesh_renderer) = mesh_renderer {
                    mesh_renderer.call(
                        "set_surface_override_material".into(),
                        &[0.to_variant(), Variant::nil()],
                    );
                    node.material.take();
                }
            }
        }

        scene.dirty_materials = true;
    }

    if scene.dirty_materials {
        let mut keep_dirty = false;
        let mut dead_materials = HashSet::with_capacity(scene.materials.capacity());
        let mut no_more_waiting_materials = HashSet::new();

        for (dcl_material, item) in scene.materials.iter() {
            if item.waiting_textures {
                let material_item = item.weak_ref.call("get_ref", &[]);
                if material_item.is_nil() {
                    // item.alive = false;
                    dead_materials.insert(dcl_material.clone());
                    continue;
                }

                let mut material = material_item.to::<Gd<StandardMaterial3D>>();
                let mut ready = true;

                match dcl_material {
                    DclMaterial::Unlit(unlit_material) => {
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::TEXTURE_ALBEDO,
                            &unlit_material.texture,
                            &mut material,
                            &mut content_manager,
                            scene,
                        );
                    }
                    DclMaterial::Pbr(pbr) => {
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::TEXTURE_ALBEDO,
                            &pbr.texture,
                            &mut material,
                            &mut content_manager,
                            scene,
                        );
                        // check_texture(
                        //     godot::engine::base_material_3d::TextureParam::,
                        //     &pbr.alpha_texture,
                        //     item,
                        //     &mut content_manager,
                        // );
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::TEXTURE_NORMAL,
                            &pbr.bump_texture,
                            &mut material,
                            &mut content_manager,
                            scene,
                        );
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::TEXTURE_EMISSION,
                            &pbr.emissive_texture,
                            &mut material,
                            &mut content_manager,
                            scene,
                        );
                    }
                }

                if !ready {
                    keep_dirty = true;
                } else {
                    // item.waiting_textures = false;
                    no_more_waiting_materials.insert(dcl_material.clone());
                }
            }
        }

        for materials in no_more_waiting_materials {
            scene
                .materials
                .get_mut(&materials)
                .unwrap()
                .waiting_textures = false;
        }

        scene.materials.retain(|k, _| !dead_materials.contains(k));
        scene.dirty_materials = keep_dirty;
    }
}

fn check_texture(
    param: godot::engine::base_material_3d::TextureParam,
    dcl_texture: &Option<DclTexture>,
    material: &mut Gd<StandardMaterial3D>,
    content_manager: &mut Node,
    scene: &Scene,
) -> bool {
    if dcl_texture.is_none() {
        return true;
    }

    let dcl_texture = dcl_texture.as_ref().unwrap();

    match &dcl_texture.source {
        DclSourceTex::Texture(content_hash) => {
            let is_loaded = content_manager
                .call(
                    "is_resource_from_hash_loaded".into(),
                    &[GodotString::from(content_hash).to_variant()],
                )
                .to::<bool>();

            if is_loaded {
                let resource = content_manager
                    .call(
                        "get_resource_from_hash".into(),
                        &[GodotString::from(content_hash).to_variant()],
                    )
                    .to::<Gd<ImageTexture>>();
                material.set_texture(param, resource.upcast());
                return true;
            } else {
                return false;
            }
        }
        DclSourceTex::AvatarTexture(_user_id) => {
            // TODO: implement load avatar texture
        }
        DclSourceTex::VideoTexture(video_entity_id) => {
            if let Some(node) = scene.godot_dcl_scene.get_node(video_entity_id) {
                if let Some(data) = &node.video_player_data {
                    material.set_texture(param, data.video_sink.tex.share().upcast());
                    return true;
                }
            }
            return false;
            // TODO: implement link video texture with entity
        }
    }

    true
}
