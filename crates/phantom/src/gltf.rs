// .gltf / .glb
pub fn import_gltf_file(path: impl AsRef<std::path::Path>) -> crate::world::World {
    let (gltf, buffers, raw_images) = gltf::import(path.as_ref()).expect("Failed to import gltf");
    import_gltf(raw_images, gltf, buffers)
}

// .glb only
pub fn import_gltf_slice(bytes: &[u8]) -> crate::world::World {
    let (gltf, buffers, raw_images) = gltf::import_slice(bytes).expect("Failed to import gltf");
    import_gltf(raw_images, gltf, buffers)
}

fn import_gltf(
    raw_images: Vec<gltf::image::Data>,
    gltf: gltf::Document,
    buffers: Vec<gltf::buffer::Data>,
) -> crate::world::World {
    let images = raw_images.into_iter().map(map_image).collect::<Vec<_>>();
    let samplers = gltf.samplers().map(map_sampler).collect::<Vec<_>>();
    let textures = gltf
        .textures()
        .map(|texture| crate::world::Texture {
            image_index: texture.source().index(),
            sampler_index: texture.sampler().index(),
        })
        .collect::<Vec<_>>();
    let materials = gltf
        .materials()
        .map(|material| crate::world::Material {
            base_color_factor: nalgebra_glm::Vec4::from(
                material.pbr_metallic_roughness().base_color_factor(),
            ),
            alpha_mode: map_alpha_mode(material.alpha_mode()),
            alpha_cutoff: material.alpha_cutoff(),
            base_color_texture_index: material
                .pbr_metallic_roughness()
                .base_color_texture()
                .map(|texture| texture.texture().index())
                .unwrap_or_default(),
            emissive_factor: material.emissive_factor().into(),
            emissive_texture_index: material
                .emissive_texture()
                .map(|texture| texture.texture().index())
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    let (meshes, vertices, indices) = {
        let (mut vertices, mut indices) = (vec![], vec![]);
        let meshes = gltf
            .meshes()
            .map(|mesh| {
                crate::world::Mesh {
                    primitives: mesh
                        .primitives()
                        .map(|primitive| {
                            let primitive_vertices: Vec<crate::world::Vertex> = {
                                let reader =
                                    primitive.reader(|buffer| Some(&*buffers[buffer.index()]));

                                let mut positions = Vec::new();
                                let read_positions = reader
                                    .read_positions()
                                    .expect("Failed to read gltf vertex positions");
                                read_positions.for_each(|position| {
                                    positions.push(nalgebra_glm::Vec3::from(position));
                                });
                                let number_of_vertices = positions.len();
                                let normals = reader.read_normals().map_or(
                                    vec![nalgebra_glm::vec3(0.0, 0.0, 0.0); number_of_vertices],
                                    |normals| {
                                        normals.map(nalgebra_glm::Vec3::from).collect::<Vec<_>>()
                                    },
                                );
                                let map_to_vec2 =
                            |coords: gltf::mesh::util::ReadTexCoords| -> Vec<nalgebra_glm::Vec2> {
                                coords
                                    .into_f32()
                                    .map(nalgebra_glm::Vec2::from)
                                    .collect::<Vec<_>>()
                            };
                                let uv_0 = reader.read_tex_coords(0).map_or(
                                    vec![nalgebra_glm::vec2(0.0, 0.0); number_of_vertices],
                                    map_to_vec2,
                                );
                                let uv_1 = reader.read_tex_coords(1).map_or(
                                    vec![nalgebra_glm::vec2(0.0, 0.0); number_of_vertices],
                                    map_to_vec2,
                                );
                                let convert_joints =
                                |joints: gltf::mesh::util::ReadJoints| -> Vec<nalgebra_glm::Vec4> {
                                    joints
                                        .into_u16()
                                        .map(|joint| {
                                            nalgebra_glm::vec4(
                                                joint[0] as _,
                                                joint[1] as _,
                                                joint[2] as _,
                                                joint[3] as _,
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                };
                                let joints_0 = reader.read_joints(0).map_or(
                                    vec![
                                        nalgebra_glm::vec4(0.0, 0.0, 0.0, 0.0);
                                        number_of_vertices
                                    ],
                                    convert_joints,
                                );
                                let convert_weights =
                            |weights: gltf::mesh::util::ReadWeights| -> Vec<nalgebra_glm::Vec4> {
                                weights.into_f32().map(nalgebra_glm::Vec4::from).collect()
                            };
                                let weights_0 = reader.read_weights(0).map_or(
                                    vec![
                                        nalgebra_glm::vec4(1.0, 0.0, 0.0, 0.0);
                                        number_of_vertices
                                    ],
                                    convert_weights,
                                );
                                let convert_colors =
                                |colors: gltf::mesh::util::ReadColors| -> Vec<nalgebra_glm::Vec3> {
                                    colors
                                        .into_rgb_f32()
                                        .map(nalgebra_glm::Vec3::from)
                                        .collect::<Vec<_>>()
                                };
                                let colors_0 = reader.read_colors(0).map_or(
                                    vec![nalgebra_glm::vec3(1.0, 1.0, 1.0); number_of_vertices],
                                    convert_colors,
                                );

                                // every vertex is guaranteed to have a position attribute,
                                // so we can use the position attribute array to index into the other attribute arrays

                                positions
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, position)| crate::world::Vertex {
                                        position,
                                        normal: normals[index],
                                        uv_0: uv_0[index],
                                        uv_1: uv_1[index],
                                        joint_0: joints_0[index],
                                        weight_0: weights_0[index],
                                        color_0: colors_0[index],
                                    })
                                    .collect()
                            };

                            let primitive_indices: Vec<u32> = primitive
                                .reader(|buffer| Some(&*buffers[buffer.index()]))
                                .read_indices()
                                .take()
                                .map(|read_indices| read_indices.into_u32().collect())
                                .unwrap_or_default();

                            let primitive = crate::world::Primitive {
                                topology: map_mesh_mode(primitive.mode()),
                                material_index: primitive.material().index(),
                                vertex_offset: vertices.len(),
                                index_offset: indices.len(),
                                number_of_vertices: primitive_vertices.len(),
                                number_of_indices: primitive_indices.len(),
                            };

                            vertices.extend(primitive_vertices);
                            indices.extend(primitive_indices);

                            primitive
                        })
                        .collect::<Vec<_>>(),
                }
            })
            .collect::<Vec<_>>();
        (meshes, vertices, indices)
    };

    let (scenes, nodes, transforms, metadata) = {
        let mut nodes = Vec::new();
        let mut transforms = Vec::new();
        let mut metadata = Vec::new();
        let scenes = gltf
            .scenes()
            .map(|gltf_scene| {
                fn visit_node(
                    parent_graph_node_index: Option<petgraph::graph::NodeIndex>,
                    node: &gltf::Node,
                    scene: &mut crate::world::Scene,
                    nodes: &mut Vec<crate::world::Node>,
                    transforms: &mut Vec<crate::world::Transform>,
                    metadata: &mut Vec<crate::world::NodeMetadata>,
                ) {
                    let transform_index = transforms.len();
                    transforms.push(crate::world::Transform::from(node.transform().decomposed()));

                    let metadata_index = metadata.len();
                    metadata.push(crate::world::NodeMetadata {
                        name: node.name().unwrap_or("Node").to_string(),
                    });

                    let node_index = nodes.len();
                    nodes.push(crate::world::Node {
                        metadata_index,
                        transform_index,
                        camera_index: node.camera().map(|camera| camera.index()),
                        mesh_index: node.mesh().map(|mesh| mesh.index()),
                        light_index: node.light().map(|light| light.index()),
                        ..Default::default()
                    });
                    let graph_node_index = scene.graph.add_node(node_index);
                    if let Some(parent_graph_node_index) = parent_graph_node_index {
                        if parent_graph_node_index != graph_node_index {
                            scene
                                .graph
                                .add_edge(parent_graph_node_index, graph_node_index, ());
                        }
                    }
                    node.children().for_each(|child| {
                        visit_node(
                            Some(graph_node_index),
                            &child,
                            scene,
                            nodes,
                            transforms,
                            metadata,
                        );
                    });
                }

                let mut scene = crate::world::Scene::default();

                let transform_index = transforms.len();
                transforms.push(crate::world::Transform::default());

                let metadata_index = metadata.len();
                metadata.push(crate::world::NodeMetadata {
                    name: "Scene Root".to_string(),
                });

                let node_index = nodes.len();
                nodes.push(crate::world::Node {
                    transform_index,
                    metadata_index,
                    ..Default::default()
                });

                let root_node_index = scene.graph.add_node(node_index);
                gltf_scene.nodes().for_each(|root_node| {
                    visit_node(
                        Some(root_node_index),
                        &root_node,
                        &mut scene,
                        &mut nodes,
                        &mut transforms,
                        &mut metadata,
                    );
                });
                scene
            })
            .collect::<Vec<_>>();
        (scenes, nodes, transforms, metadata)
    };

    let skins = gltf
        .skins()
        .map(|gltf_skin| {
            let reader = gltf_skin.reader(|buffer| Some(&buffers[buffer.index()]));
            let inverse_bind_matrices = reader
                .read_inverse_bind_matrices()
                .map_or(Vec::new(), |matrices| {
                    matrices.map(nalgebra_glm::Mat4::from).collect::<Vec<_>>()
                });
            let joints = gltf_skin
                .joints()
                .enumerate()
                .map(|(index, joint_node)| {
                    let inverse_bind_matrix = *inverse_bind_matrices
                        .get(index)
                        .unwrap_or(&nalgebra_glm::Mat4::identity());
                    crate::world::Joint {
                        inverse_bind_matrix,
                        target_node_index: joint_node.index(),
                    }
                })
                .collect();
            crate::world::Skin { joints }
        })
        .collect::<Vec<_>>();

    let animations = gltf
        .animations()
        .map(|animation| {
            let channels = animation
                .channels()
                .map(|channel| {
                    let target_node_index = channel.target().node().index();
                    let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                    let inputs = reader
                        .read_inputs()
                        .expect("Failed to read animation channel inputs!")
                        .collect::<Vec<_>>();
                    let outputs = reader
                        .read_outputs()
                        .expect("Failed to read animation channel outputs!");
                    let transformations = match outputs {
                        gltf::animation::util::ReadOutputs::Translations(translations) => {
                            let translations = translations
                                .map(nalgebra_glm::Vec3::from)
                                .collect::<Vec<_>>();
                            crate::world::TransformationSet::Translations(translations)
                        }
                        gltf::animation::util::ReadOutputs::Rotations(rotations) => {
                            let rotations = rotations
                                .into_f32()
                                .map(nalgebra_glm::Vec4::from)
                                .collect::<Vec<_>>();
                            crate::world::TransformationSet::Rotations(rotations)
                        }
                        gltf::animation::util::ReadOutputs::Scales(scales) => {
                            let scales = scales.map(nalgebra_glm::Vec3::from).collect::<Vec<_>>();
                            crate::world::TransformationSet::Scales(scales)
                        }
                        gltf::animation::util::ReadOutputs::MorphTargetWeights(weights) => {
                            let morph_target_weights = weights.into_f32().collect::<Vec<_>>();
                            crate::world::TransformationSet::MorphTargetWeights(
                                morph_target_weights,
                            )
                        }
                    };
                    crate::world::Channel {
                        target_node_index,
                        inputs,
                        transformations,
                        interpolation: crate::world::Interpolation::default(),
                    }
                })
                .collect::<Vec<_>>();
            let max_animation_time = channels
                .iter()
                .flat_map(|channel| channel.inputs.iter().copied())
                .fold(0.0, f32::max);
            crate::world::Animation {
                channels,
                time: 0.0,
                max_animation_time,
            }
        })
        .collect::<Vec<_>>();

    let lights = match gltf.lights() {
        Some(lights) => lights.into_iter().map(map_light).collect(),
        None => vec![],
    };

    let cameras = gltf.cameras().map(map_camera).collect::<Vec<_>>();

    crate::world::World {
        animations,
        cameras,
        images,
        indices,
        lights,
        materials,
        meshes,
        nodes,
        metadata,
        samplers,
        scenes,
        skins,
        textures,
        transforms,
        vertices,
        ..Default::default()
    }
}

fn map_alpha_mode(mode: gltf::material::AlphaMode) -> crate::world::AlphaMode {
    match mode {
        gltf::material::AlphaMode::Opaque => crate::world::AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => crate::world::AlphaMode::Mask,
        gltf::material::AlphaMode::Blend => crate::world::AlphaMode::Blend,
    }
}

fn map_sampler(gltf_sampler: gltf::texture::Sampler) -> crate::world::Sampler {
    let min_filter = gltf_sampler
        .min_filter()
        .map(|filter| match filter {
            gltf::texture::MinFilter::Nearest => crate::world::MinFilter::Nearest,
            gltf::texture::MinFilter::NearestMipmapNearest => {
                crate::world::MinFilter::NearestMipmapNearest
            }
            gltf::texture::MinFilter::LinearMipmapNearest => {
                crate::world::MinFilter::LinearMipmapNearest
            }
            gltf::texture::MinFilter::Linear => crate::world::MinFilter::Linear,
            gltf::texture::MinFilter::LinearMipmapLinear => {
                crate::world::MinFilter::LinearMipmapLinear
            }
            gltf::texture::MinFilter::NearestMipmapLinear => {
                crate::world::MinFilter::NearestMipmapLinear
            }
        })
        .unwrap_or_default();

    let mag_filter = gltf_sampler
        .mag_filter()
        .map(|filter| match filter {
            gltf::texture::MagFilter::Linear => crate::world::MagFilter::Linear,
            gltf::texture::MagFilter::Nearest => crate::world::MagFilter::Nearest,
        })
        .unwrap_or_default();

    let wrap_s = match gltf_sampler.wrap_s() {
        gltf::texture::WrappingMode::ClampToEdge => crate::world::WrappingMode::ClampToEdge,
        gltf::texture::WrappingMode::MirroredRepeat => crate::world::WrappingMode::MirroredRepeat,
        gltf::texture::WrappingMode::Repeat => crate::world::WrappingMode::Repeat,
    };

    let wrap_t = match gltf_sampler.wrap_t() {
        gltf::texture::WrappingMode::ClampToEdge => crate::world::WrappingMode::ClampToEdge,
        gltf::texture::WrappingMode::MirroredRepeat => crate::world::WrappingMode::MirroredRepeat,
        gltf::texture::WrappingMode::Repeat => crate::world::WrappingMode::Repeat,
    };

    crate::world::Sampler {
        min_filter,
        mag_filter,
        wrap_s,
        wrap_t,
    }
}

fn map_image(data: gltf::image::Data) -> crate::world::Image {
    let img = match data.format {
        gltf::image::Format::R8 => image::DynamicImage::ImageLuma8(
            image::ImageBuffer::from_raw(data.width, data.height, data.pixels.to_vec()).unwrap(),
        ),
        gltf::image::Format::R8G8 => image::DynamicImage::ImageLumaA8(
            image::ImageBuffer::from_raw(data.width, data.height, data.pixels.to_vec()).unwrap(),
        ),
        gltf::image::Format::R8G8B8 => image::DynamicImage::ImageRgb8(
            image::ImageBuffer::from_raw(data.width, data.height, data.pixels.to_vec()).unwrap(),
        ),
        gltf::image::Format::R8G8B8A8 => image::DynamicImage::ImageRgba8(
            image::ImageBuffer::from_raw(data.width, data.height, data.pixels.to_vec()).unwrap(),
        ),
        format => panic!("Unsupported image format: {format:#?}"),
    };
    let rgba_img = img.to_rgba8();
    let pixels = rgba_img.into_raw();
    crate::world::Image {
        pixels,
        format: crate::world::ImageFormat::R8G8B8A8,
        width: data.width,
        height: data.height,
    }
}

fn map_camera(camera: gltf::Camera) -> crate::world::Camera {
    crate::world::Camera {
        projection: match camera.projection() {
            gltf::camera::Projection::Perspective(camera) => {
                crate::world::Projection::Perspective(crate::world::PerspectiveCamera {
                    aspect_ratio: camera.aspect_ratio(),
                    y_fov_rad: camera.yfov(),
                    z_far: camera.zfar(),
                    z_near: camera.znear(),
                })
            }
            gltf::camera::Projection::Orthographic(camera) => {
                crate::world::Projection::Orthographic(crate::world::OrthographicCamera {
                    x_mag: camera.xmag(),
                    y_mag: camera.ymag(),
                    z_far: camera.zfar(),
                    z_near: camera.znear(),
                })
            }
        },
    }
}

fn map_light(light: gltf::khr_lights_punctual::Light) -> crate::world::Light {
    crate::world::Light {
        color: light.color().into(),
        intensity: light.intensity(),
        range: light.range().unwrap_or(0.0),
        kind: map_light_kind(light.kind()),
    }
}

fn map_light_kind(kind: gltf::khr_lights_punctual::Kind) -> crate::world::LightKind {
    match kind {
        gltf::khr_lights_punctual::Kind::Directional => crate::world::LightKind::Directional,
        gltf::khr_lights_punctual::Kind::Point => crate::world::LightKind::Point,
        gltf::khr_lights_punctual::Kind::Spot {
            inner_cone_angle,
            outer_cone_angle,
        } => crate::world::LightKind::Spot {
            inner_cone_angle,
            outer_cone_angle,
        },
    }
}

fn map_mesh_mode(mode: gltf::mesh::Mode) -> crate::world::PrimitiveTopology {
    match mode {
        gltf::mesh::Mode::Points => crate::world::PrimitiveTopology::Points,
        gltf::mesh::Mode::Lines => crate::world::PrimitiveTopology::Lines,
        gltf::mesh::Mode::LineStrip => crate::world::PrimitiveTopology::LineStrip,
        gltf::mesh::Mode::TriangleStrip => crate::world::PrimitiveTopology::TriangleStrip,
        gltf::mesh::Mode::LineLoop => crate::world::PrimitiveTopology::LineLoop,
        gltf::mesh::Mode::TriangleFan => crate::world::PrimitiveTopology::TriangleFan,
        gltf::mesh::Mode::Triangles => crate::world::PrimitiveTopology::Triangles,
    }
}
