use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{pb_mesh_renderer, PbMeshRenderer},
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::{
    engine::{BoxMesh, MeshInstance3D},
    prelude::*,
};

pub fn create_or_update_mesh(mesh_instance: &mut Gd<MeshInstance3D>, mesh: &PbMeshRenderer) {
    let current_mesh = mesh_instance.get_mesh();

    match mesh.mesh.as_ref() {
        Some(mesh) => match mesh {
            pb_mesh_renderer::Mesh::Box(box_mesh) => {
                let uvs = box_mesh
                    .uvs
                    .iter()
                    .map(f32::to_variant)
                    .collect::<VariantArray>()
                    .to_variant();
                mesh_instance.call("set_box".into(), &[uvs]);
            }
            pb_mesh_renderer::Mesh::Sphere(_) => {
                mesh_instance.call("set_sphere".into(), &[]);
            }
            pb_mesh_renderer::Mesh::Cylinder(cylinder_mesh_value) => {
                let top_radius = cylinder_mesh_value.radius_top.unwrap_or(0.5);
                let bottom_radius = cylinder_mesh_value.radius_bottom.unwrap_or(0.5);
                mesh_instance.call(
                    "set_cylinder".into(),
                    &[top_radius.to_variant(), bottom_radius.to_variant()],
                );
            }
            pb_mesh_renderer::Mesh::Plane(plane_mesh) => {
                let uvs = plane_mesh
                    .uvs
                    .iter()
                    .map(f32::to_variant)
                    .collect::<VariantArray>()
                    .to_variant();

                mesh_instance.call("set_plane".into(), &[uvs]);
            }
        },
        _ => {
            let box_mesh = match current_mesh {
                Some(current_mesh) => current_mesh.try_cast::<BoxMesh>().unwrap_or(BoxMesh::new()),
                None => BoxMesh::new(),
            };
            mesh_instance.set_mesh(box_mesh.upcast());
            // update the material (and with uvs)
        }
    }
}

pub fn update_mesh_renderer(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(mesh_renderer_dirty) = dirty_lww_components.get(&SceneComponentId::MESH_RENDERER) {
        let mesh_renderer_component = SceneCrdtStateProtoComponents::get_mesh_renderer(crdt_state);

        for entity in mesh_renderer_dirty {
            let new_value = mesh_renderer_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();
            let existing = node
                .base
                .try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

            if new_value.is_none() {
                if let Some(mesh_renderer_node) = existing {
                    node.base.remove_child(mesh_renderer_node.upcast());
                }
            } else if let Some(new_value) = new_value {
                let (mut mesh_instance_3d, add_to_base) = match existing {
                    Some(mesh_instance_3d) => (mesh_instance_3d, false),
                    None => (
                        godot::engine::load::<PackedScene>(
                            "res://src/decentraland_components/mesh_renderer.tscn",
                        )
                        .instantiate()
                        .unwrap()
                        .cast::<MeshInstance3D>(),
                        true,
                    ),
                };

                create_or_update_mesh(&mut mesh_instance_3d, &new_value);

                if add_to_base {
                    mesh_instance_3d.set_name(GodotString::from("MeshRenderer"));
                    node.base.add_child(mesh_instance_3d.upcast());
                }
            }
        }
    }
}
