use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gltf;
use image::{DynamicImage, ImageError, ImageOutputFormat, RgbaImage};
use runvy_asset::{Handle, TextureAsset};
use runvy_core::components::{AlphaMode, Material, Mesh, MeshRenderer, Transform, Vertex3D};
use runvy_core::glam::{Quat, Vec3};
use runvy_core::ocs::{Object, ObjectId};
use runvy_core::World;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zip::write::FileOptions;

const RUNVY3D_FORMAT: &str = "r3m";
const RUNVY3D_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum Runvy3DError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("glTF error: {0}")]
    Gltf(#[from] gltf::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Unsupported import format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid r3m file: {0}")]
    InvalidFormat(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSettings {
    pub scale: f32,
    pub import_materials: bool,
    pub import_textures: bool,
    pub import_animations: bool,
    pub generate_tangents: bool,
    pub use_vertex_colors: bool,
    pub optimize_mesh: bool,
    pub merge_meshes: bool,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            scale: 1.0,
            import_materials: true,
            import_textures: true,
            import_animations: false,
            generate_tangents: false,
            use_vertex_colors: true,
            optimize_mesh: false,
            merge_meshes: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DModelJson {
    pub format: String,
    pub version: u32,
    pub guid: String,
    pub name: String,
    pub nodes: Vec<Runvy3DNode>,
    pub meshes: Vec<Runvy3DMeshEntry>,
    pub materials: Vec<Runvy3DMaterialEntry>,
    pub textures: Vec<Runvy3DTextureEntry>,
    pub animations: Vec<Runvy3DAnimationEntry>,
    pub skeletons: Vec<Runvy3DSkeletonEntry>,
    pub metadata: Runvy3DMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DMetadata {
    pub source: Option<String>,
    pub import_settings: ImportSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DNode {
    pub id: u32,
    pub name: String,
    pub parent: Option<u32>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub mesh: Option<u32>,
    pub material_slots: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DMeshEntry {
    pub id: u32,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DMaterialEntry {
    pub id: u32,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DTextureEntry {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub color_space: Runvy3DColorSpace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DAnimationEntry {
    pub id: u32,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DSkeletonEntry {
    pub id: u32,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Runvy3DColorSpace {
    Srgb,
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runvy3DMaterialJson {
    pub name: String,
    pub base_color: [f32; 4],
    pub base_color_texture: Option<u32>,
    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: Option<u32>,
    pub normal_texture: Option<u32>,
    pub occlusion_texture: Option<u32>,
    pub emission: [f32; 3],
    pub emissive_texture: Option<u32>,
    pub alpha_mode: Runvy3DAlphaMode,
    pub double_sided: bool,
    pub use_vertex_color: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Runvy3DAlphaMode {
    Opaque,
    Mask,
    Blend,
}

#[derive(Debug, Clone)]
pub struct RunvyModel {
    pub manifest: Runvy3DModelJson,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub fn import_to_runvy3d(
    source: &Path,
    output: &Path,
    settings: ImportSettings,
) -> Result<(), Runvy3DError> {
    let extension = source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    match extension.as_deref() {
        Some("glb") | Some("gltf") => import_gltf_to_runvy3d(source, output, settings),
        Some(ext) => Err(Runvy3DError::UnsupportedFormat(ext.to_string())),
        None => Err(Runvy3DError::UnsupportedFormat(
            "missing file extension".to_string(),
        )),
    }
}

pub fn import_glb_to_runvy3d(
    source: &Path,
    output: &Path,
    settings: ImportSettings,
) -> Result<(), Runvy3DError> {
    import_gltf_to_runvy3d(source, output, settings)
}

fn import_gltf_to_runvy3d(
    source: &Path,
    output: &Path,
    settings: ImportSettings,
) -> Result<(), Runvy3DError> {
    let (document, buffers, images) = gltf::import(source)?;

    let mut meshes = Vec::new();
    let mut materials = Vec::new();
    let mut texture_entries = Vec::new();
    let mut texture_blobs = Vec::new();
    let mut material_entries = Vec::new();
    let mut mesh_entries = Vec::new();

    if settings.import_materials {
        for material in document.materials() {
            let pbr = material.pbr_metallic_roughness();
            let id = materials.len() as u32;
            materials.push(Runvy3DMaterialJson {
                name: material.name().unwrap_or("Material").to_string(),
                base_color: pbr.base_color_factor(),
                base_color_texture: if settings.import_textures {
                    pbr.base_color_texture()
                        .map(|info| info.texture().index() as u32)
                } else {
                    None
                },
                metallic: pbr.metallic_factor(),
                roughness: pbr.roughness_factor(),
                metallic_roughness_texture: if settings.import_textures {
                    pbr.metallic_roughness_texture()
                        .map(|info| info.texture().index() as u32)
                } else {
                    None
                },
                normal_texture: if settings.import_textures {
                    material
                        .normal_texture()
                        .map(|info| info.texture().index() as u32)
                } else {
                    None
                },
                occlusion_texture: if settings.import_textures {
                    material
                        .occlusion_texture()
                        .map(|info| info.texture().index() as u32)
                } else {
                    None
                },
                emission: material.emissive_factor(),
                emissive_texture: if settings.import_textures {
                    material
                        .emissive_texture()
                        .map(|info| info.texture().index() as u32)
                } else {
                    None
                },
                alpha_mode: match material.alpha_mode() {
                    gltf::material::AlphaMode::Opaque => Runvy3DAlphaMode::Opaque,
                    gltf::material::AlphaMode::Mask => Runvy3DAlphaMode::Mask,
                    gltf::material::AlphaMode::Blend => Runvy3DAlphaMode::Blend,
                },
                double_sided: material.double_sided(),
                use_vertex_color: settings.use_vertex_colors,
            });
            material_entries.push(Runvy3DMaterialEntry {
                id,
                name: material.name().unwrap_or("Material").to_string(),
                path: format!("materials/{id}.json"),
            });
        }
    }

    if settings.import_textures {
        for texture in document.textures() {
            let id = texture.index() as u32;
            let image = &images[texture.source().index()];
            let png = image_to_png_bytes(image)?;
            texture_entries.push(Runvy3DTextureEntry {
                id,
                name: texture.name().unwrap_or("Texture").to_string(),
                path: format!("textures/{id}.png"),
                color_space: Runvy3DColorSpace::Srgb,
            });
            texture_blobs.push(Runvy3DTextureBlob { bytes: png });
        }
    }

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<_> = positions.collect();
            let normals: Vec<_> = reader
                .read_normals()
                .map(|normals| normals.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|coords| coords.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
            let colors: Vec<[f32; 4]> = if settings.use_vertex_colors {
                reader
                    .read_colors(0)
                    .map(|colors| colors.into_rgba_f32().collect())
                    .unwrap_or_else(|| vec![[1.0, 1.0, 1.0, 1.0]; positions.len()])
            } else {
                vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]
            };
            let indices: Vec<u32> = reader
                .read_indices()
                .map(|indices| indices.into_u32().collect())
                .unwrap_or_else(|| (0..positions.len() as u32).collect());

            let vertices = positions
                .iter()
                .enumerate()
                .map(|(index, position)| Runvy3DVertex {
                    position: [
                        position[0] * settings.scale,
                        position[1] * settings.scale,
                        position[2] * settings.scale,
                    ],
                    normal: normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]),
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    texcoord_0: uvs.get(index).copied().unwrap_or([0.0, 0.0]),
                    texcoord_1: [0.0, 0.0],
                    color_0: colors.get(index).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]),
                    joints_0: [0, 0, 0, 0],
                    weights_0: [0.0, 0.0, 0.0, 0.0],
                })
                .collect();

            let id = meshes.len() as u32;
            meshes.push(Runvy3DMeshBlob { vertices, indices });
            mesh_entries.push(Runvy3DMeshEntry {
                id,
                name: mesh.name().unwrap_or("Mesh").to_string(),
                path: format!("meshes/{id}.bin"),
            });
        }
    }

    let mut nodes = Vec::new();
    for node in document.nodes() {
        let (translation, rotation, scale) = node.transform().decomposed();
        let mesh_id = node.mesh().and_then(|mesh| {
            mesh.primitives()
                .next()
                .map(|primitive| primitive.index() as u32)
        });
        nodes.push(Runvy3DNode {
            id: node.index() as u32,
            name: node.name().unwrap_or("Node").to_string(),
            parent: None,
            translation: [
                translation[0] * settings.scale,
                translation[1] * settings.scale,
                translation[2] * settings.scale,
            ],
            rotation,
            scale,
            mesh: mesh_id,
            material_slots: node
                .mesh()
                .map(|mesh| {
                    mesh.primitives()
                        .filter_map(|primitive| {
                            primitive.material().index().map(|index| index as u32)
                        })
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    let name = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Model")
        .to_string();
    let manifest = Runvy3DModelJson {
        format: RUNVY3D_FORMAT.to_string(),
        version: RUNVY3D_VERSION,
        guid: generate_guid_like_string(source),
        name,
        nodes,
        meshes: mesh_entries,
        materials: material_entries,
        textures: texture_entries,
        animations: Vec::new(),
        skeletons: Vec::new(),
        metadata: Runvy3DMetadata {
            source: Some(source.to_string_lossy().to_string()),
            import_settings: settings,
        },
    };

    write_runvy3d(output, &manifest, &meshes, &materials, &texture_blobs)
}

fn load_image_bytes(path: &Path) -> Result<Vec<u8>, ImageError> {
    let image = image::open(path)?.to_rgba8();
    let mut bytes = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Png)?;
    Ok(bytes)
}

fn image_to_png_bytes(image: &gltf::image::Data) -> Result<Vec<u8>, Runvy3DError> {
    let pixels = match image.format {
        gltf::image::Format::R8G8B8 => {
            let mut rgba = Vec::with_capacity(image.pixels.len() / 3 * 4);
            for chunk in image.pixels.chunks_exact(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            rgba
        }
        gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
        _ => {
            return Err(Runvy3DError::InvalidFormat(format!(
                "unsupported glTF image format: {:?}",
                image.format
            )))
        }
    };

    let width = image.width;
    let height = image.height;
    let rgba = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| Runvy3DError::InvalidFormat("invalid glTF image dimensions".into()))?;
    let image = DynamicImage::ImageRgba8(rgba);

    let mut png = Vec::new();
    image.write_to(&mut Cursor::new(&mut png), ImageOutputFormat::Png)?;
    Ok(png)
}

pub fn load_runvy3d(path: &Path) -> Result<RunvyModel, Runvy3DError> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let manifest: Runvy3DModelJson = read_json_from_zip(&mut archive, "model.json")?;
    if manifest.format != RUNVY3D_FORMAT {
        return Err(Runvy3DError::InvalidFormat(format!(
            "{} is not a .r3m file",
            path.display()
        )));
    }

    let mut textures = Vec::new();
    for entry in &manifest.textures {
        let mut file = archive.by_name(&entry.path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        textures.push(Handle::from(Arc::new(TextureAsset::from_rgba8(
            PathBuf::from(&entry.path),
            image.width(),
            image.height(),
            image.into_raw(),
        ))));
    }

    let mut meshes = Vec::new();
    for entry in &manifest.meshes {
        let mut file = archive.by_name(&entry.path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        meshes.push(read_mesh_blob(&bytes)?.into_mesh());
    }

    let mut materials = Vec::new();
    for entry in &manifest.materials {
        let material: Runvy3DMaterialJson = read_json_from_zip(&mut archive, &entry.path)?;
        materials.push(material.into_material(&textures));
    }

    Ok(RunvyModel {
        manifest,
        meshes,
        materials,
    })
}

pub fn spawn_runvy_model(world: &mut World, model: &RunvyModel) -> Vec<ObjectId> {
    let mut spawned = HashMap::new();
    let mut ids = Vec::new();
    for node in &model.manifest.nodes {
        let mut object = Object::new(node.name.clone());
        let mut transform = Transform::default();
        transform.position = Vec3::from_array(node.translation);
        transform.rotation = Quat::from_array(node.rotation);
        transform.scale = Vec3::from_array(node.scale);
        object.add_component(transform);

        if let Some(mesh_id) = node.mesh {
            if let Some(mesh) = model.meshes.get(mesh_id as usize) {
                let mut renderer = MeshRenderer::new(mesh.clone());
                renderer.materials = node
                    .material_slots
                    .iter()
                    .filter_map(|slot| model.materials.get(*slot as usize))
                    .cloned()
                    .map(|material| Handle::from(Arc::new(material)))
                    .collect();
                if renderer.materials.is_empty() {
                    renderer
                        .materials
                        .push(Handle::from(Arc::new(Material::default())));
                }
                object.add_component(renderer);
            }
        }

        let id = world.spawn(object);
        spawned.insert(node.id, id);
        ids.push(id);
    }

    for node in &model.manifest.nodes {
        if let (Some(child), Some(parent_node)) = (spawned.get(&node.id), node.parent) {
            if let Some(parent) = spawned.get(&parent_node) {
                world.set_parent(*child, Some(*parent));
            }
        }
    }

    ids
}

fn write_runvy3d(
    output: &Path,
    manifest: &Runvy3DModelJson,
    meshes: &[Runvy3DMeshBlob],
    materials: &[Runvy3DMaterialJson],
    textures: &[Runvy3DTextureBlob],
) -> Result<(), Runvy3DError> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("model.json", options)?;
    zip.write_all(serde_json::to_string_pretty(manifest)?.as_bytes())?;

    for (index, mesh) in meshes.iter().enumerate() {
        zip.start_file(format!("meshes/{index}.bin"), options)?;
        zip.write_all(&mesh.to_bytes())?;
    }
    for (index, material) in materials.iter().enumerate() {
        zip.start_file(format!("materials/{index}.json"), options)?;
        zip.write_all(serde_json::to_string_pretty(material)?.as_bytes())?;
    }
    for (index, texture) in textures.iter().enumerate() {
        zip.start_file(format!("textures/{index}.png"), options)?;
        zip.write_all(&texture.bytes)?;
    }
    zip.start_file("editor.json", options)?;
    zip.write_all(br#"{"version":1}"#)?;
    zip.start_file("thumbnail.png", options)?;
    zip.write_all(&[])?;
    zip.finish()?;
    Ok(())
}

fn read_json_from_zip<T: for<'de> Deserialize<'de>>(
    archive: &mut zip::ZipArchive<File>,
    path: &str,
) -> Result<T, Runvy3DError> {
    let mut file = archive.by_name(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(serde_json::from_str(&content)?)
}

#[derive(Debug, Clone)]
struct Runvy3DMeshBlob {
    vertices: Vec<Runvy3DVertex>,
    indices: Vec<u32>,
}

impl Runvy3DMeshBlob {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_u32(&mut bytes, self.vertices.len() as u32);
        for vertex in &self.vertices {
            for value in vertex.as_f32_stream() {
                write_f32(&mut bytes, value);
            }
            for joint in vertex.joints_0 {
                write_u32(&mut bytes, joint);
            }
        }
        write_u32(&mut bytes, self.indices.len() as u32);
        for index in &self.indices {
            write_u32(&mut bytes, *index);
        }
        bytes
    }

    fn into_mesh(self) -> Mesh {
        let vertices = self
            .vertices
            .into_iter()
            .map(|vertex| Vertex3D {
                position: vertex.position,
                normal: vertex.normal,
                uv: vertex.texcoord_0,
                color: vertex.color_0,
            })
            .collect();
        Mesh::new(vertices, self.indices)
    }
}

#[derive(Debug, Clone)]
struct Runvy3DVertex {
    position: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 4],
    texcoord_0: [f32; 2],
    texcoord_1: [f32; 2],
    color_0: [f32; 4],
    joints_0: [u32; 4],
    weights_0: [f32; 4],
}

impl Runvy3DVertex {
    fn as_f32_stream(&self) -> Vec<f32> {
        let mut values = Vec::with_capacity(22);
        values.extend(self.position);
        values.extend(self.normal);
        values.extend(self.tangent);
        values.extend(self.texcoord_0);
        values.extend(self.texcoord_1);
        values.extend(self.color_0);
        values.extend(self.weights_0);
        values
    }
}

fn read_mesh_blob(bytes: &[u8]) -> Result<Runvy3DMeshBlob, Runvy3DError> {
    let mut cursor = Cursor::new(bytes);
    let vertex_count = read_u32(&mut cursor)? as usize;
    let mut vertices = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let mut next = || read_f32(&mut cursor);
        let vertex = Runvy3DVertex {
            position: [next()?, next()?, next()?],
            normal: [next()?, next()?, next()?],
            tangent: [next()?, next()?, next()?, next()?],
            texcoord_0: [next()?, next()?],
            texcoord_1: [next()?, next()?],
            color_0: [next()?, next()?, next()?, next()?],
            weights_0: [next()?, next()?, next()?, next()?],
            joints_0: [
                read_u32(&mut cursor)?,
                read_u32(&mut cursor)?,
                read_u32(&mut cursor)?,
                read_u32(&mut cursor)?,
            ],
        };
        vertices.push(vertex);
    }
    let index_count = read_u32(&mut cursor)? as usize;
    let mut indices = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        indices.push(read_u32(&mut cursor)?);
    }
    Ok(Runvy3DMeshBlob { vertices, indices })
}

impl Runvy3DMaterialJson {
    fn into_material(self, textures: &[Handle<TextureAsset>]) -> Material {
        Material {
            base_color: self.base_color,
            base_color_texture: self
                .base_color_texture
                .and_then(|index| textures.get(index as usize).cloned()),
            metallic: self.metallic,
            roughness: self.roughness,
            metallic_roughness_texture: self
                .metallic_roughness_texture
                .and_then(|index| textures.get(index as usize).cloned()),
            normal_texture: self
                .normal_texture
                .and_then(|index| textures.get(index as usize).cloned()),
            occlusion_texture: self
                .occlusion_texture
                .and_then(|index| textures.get(index as usize).cloned()),
            use_vertex_color: self.use_vertex_color,
            emission: self.emission,
            emissive_texture: self
                .emissive_texture
                .and_then(|index| textures.get(index as usize).cloned()),
            alpha_mode: match self.alpha_mode {
                Runvy3DAlphaMode::Opaque => AlphaMode::Opaque,
                Runvy3DAlphaMode::Mask => AlphaMode::Mask,
                Runvy3DAlphaMode::Blend => AlphaMode::Blend,
            },
            double_sided: self.double_sided,
        }
    }
}

fn write_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_le_bytes());
}

fn write_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend(value.to_le_bytes());
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, Runvy3DError> {
    let mut bytes = [0; 4];
    cursor.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_f32(cursor: &mut Cursor<&[u8]>) -> Result<f32, Runvy3DError> {
    let mut bytes = [0; 4];
    cursor.read_exact(&mut bytes)?;
    Ok(f32::from_le_bytes(bytes))
}

fn generate_guid_like_string(source: &Path) -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{:x}-{:x}", stamp, source.to_string_lossy().len())
}

#[derive(Debug, Clone)]
struct Runvy3DTextureBlob {
    bytes: Vec<u8>,
}
