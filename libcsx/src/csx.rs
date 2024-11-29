use std::collections::HashMap;

use cgmath::{
    EuclideanSpace, InnerSpace, Matrix, Matrix3, Matrix4, Point3, Rad, Rotation3, Transform,
    Vector3,
};
use dif::interior_path_follower::{InteriorPathFollower, WayPoint};
use dif::trigger::{Polyhedron, PolyhedronEdge, Trigger};
use dif::types::{Dictionary, QuatF};
use dif::{
    dif::Dif,
    game_entity::GameEntity,
    interior::Interior,
    io::{Version, Writable},
    types::{MatrixF, PlaneF, Point3F},
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::builder::{
    get_bounding_box, get_bounding_box_not_owned, BSPReport, DIFBuilder, ProgressEventListener,
};
use crate::light::{self, Light};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConstructorScene {
    #[serde(rename = "DetailLevels")]
    pub detail_levels: DetailLevels,
    #[serde(rename = "@version")]
    pub version: i32,
    #[serde(rename = "@creator")]
    pub creator: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DetailLevels {
    pub detail_level: Vec<DetailLevel>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DetailLevel {
    pub interior_map: InteriorMap,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InteriorMap {
    #[serde(rename = "@brushScale")]
    pub brush_scale: u32,

    #[serde(rename = "@lightScale")]
    pub light_scale: u32,

    #[serde(
        rename = "@ambientColor",
        serialize_with = "serialize_point",
        deserialize_with = "deserialize_point"
    )]
    pub ambient_color: Point3F,

    #[serde(
        rename = "@ambientColorEmerg",
        serialize_with = "serialize_point",
        deserialize_with = "deserialize_point"
    )]
    pub ambient_color_emerg: Point3F,

    pub entities: Entities,
    pub brushes: Brushes,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Entities {
    pub entity: Vec<Entity>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Entity {
    #[serde(rename = "@id")]
    pub id: i32,
    #[serde(rename = "@classname")]
    pub classname: String,
    #[serde(rename = "@gametype")]
    pub gametype: String,
    #[serde(default)]
    #[serde(
        rename = "@origin",
        serialize_with = "serialize_point_optional",
        deserialize_with = "deserialize_point_optional"
    )]
    pub origin: Option<Point3F>,
    #[serde(
        serialize_with = "serialize_propertymap",
        deserialize_with = "deserialize_propertymap"
    )]
    pub properties: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EntityProperties {
    #[serde(rename = "$value", deserialize_with = "deserialize_propertymap")]
    pub property_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Brushes {
    pub brush: Vec<Brush>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Brush {
    #[serde(rename = "@id")]
    pub id: i32,
    #[serde(rename = "@owner")]
    pub owner: i32,
    #[serde(rename = "@type")]
    pub type_: i32,
    #[serde(
        rename = "@transform",
        serialize_with = "serialize_matrix",
        deserialize_with = "deserialize_matrix"
    )]
    pub transform: MatrixF,
    pub vertices: Vertices,
    pub face: Vec<Face>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Vertices {
    pub vertex: Vec<Vertex>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Vertex {
    #[serde(
        rename = "@pos",
        serialize_with = "serialize_point",
        deserialize_with = "deserialize_point"
    )]
    pub pos: Point3F,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Face {
    #[serde(rename = "@id")]
    pub id: i32,
    #[serde(
        rename = "@plane",
        serialize_with = "serialize_plane",
        deserialize_with = "deserialize_plane"
    )]
    pub plane: PlaneF,
    #[serde(rename = "@material")]
    pub material: String,
    #[serde(
        rename = "@texgens",
        serialize_with = "serialize_texgen",
        deserialize_with = "deserialize_texgen"
    )]
    pub texgens: TexGen,
    #[serde(
        rename = "@texDiv",
        serialize_with = "serialize_number_list",
        deserialize_with = "deserialize_number_list"
    )]
    pub tex_div: Vec<i32>,
    pub indices: Indices,
    #[serde(skip_deserializing)]
    pub face_id: i32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Indices {
    #[serde(
        rename = "@indices",
        serialize_with = "serialize_number_list",
        deserialize_with = "deserialize_number_list"
    )]
    pub indices: Vec<i32>,
}

#[derive(Clone)]
pub struct TexGen {
    pub plane_x: PlaneF,
    pub plane_y: PlaneF,
    pub rot: f32,
    pub scale: [f32; 2],
}

fn deserialize_point<'de, D>(deserializer: D) -> Result<Point3F, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(s) => {
            let coords = s
                .trim()
                .split(' ')
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<f32>>();
            Ok(Point3F::new(coords[0], coords[1], coords[2]))
        }
        Err(e) => Err(e),
    }
}

fn serialize_point<S>(v: &Point3F, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let format_str = format!("{} {} {}", v.x, v.y, v.z);
    format_str.serialize(serializer)
}

fn deserialize_point_optional<'de, D>(deserializer: D) -> Result<Option<Point3F>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(s) => {
            if s.len() == 0 {
                return Ok(None);
            }
            let coords = s
                .trim()
                .split(' ')
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<f32>>();
            Ok(Some(Point3F::new(coords[0], coords[1], coords[2])))
        }
        Err(e) => Err(e),
    }
}

fn serialize_point_optional<S>(v: &Option<Point3F>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match v {
        Some(v) => {
            let format_str = format!("{} {} {}", v.x, v.y, v.z);
            format_str.serialize(serializer)
        }
        None => "".serialize(serializer),
    }
}

fn deserialize_plane<'de, D>(deserializer: D) -> Result<PlaneF, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(s) => {
            let coords = s
                .trim()
                .split(' ')
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<f32>>();
            Ok(PlaneF {
                normal: Point3F::new(coords[0], coords[1], coords[2]),
                distance: coords[3],
            })
        }
        Err(e) => Err(e),
    }
}

fn serialize_plane<S>(v: &PlaneF, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let format_str = format!(
        "{} {} {} {}",
        v.normal.x, v.normal.y, v.normal.z, v.distance
    );
    format_str.serialize(serializer)
}

fn deserialize_number_list<'de, D>(deserializer: D) -> Result<Vec<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(s) => Ok(s
            .trim()
            .split(' ')
            .map(|v| v.parse::<i32>().unwrap())
            .collect()),
        Err(e) => Err(e),
    }
}

fn serialize_number_list<S>(v: &Vec<i32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let format_str = format!(
        "{}",
        v.iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    );
    format_str.serialize(serializer)
}

fn deserialize_texgen<'de, D>(deserializer: D) -> Result<TexGen, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(s) => {
            let coords = s
                .trim()
                .split(' ')
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<f32>>();
            Ok(TexGen {
                plane_x: {
                    PlaneF {
                        normal: Point3F::new(coords[0], coords[1], coords[2]),
                        distance: coords[3],
                    }
                },
                plane_y: {
                    PlaneF {
                        normal: Point3F::new(coords[4], coords[5], coords[6]),
                        distance: coords[7],
                    }
                },
                rot: coords[8],
                scale: [coords[9], coords[10]],
            })
        }
        Err(e) => Err(e),
    }
}

fn serialize_texgen<S>(v: &TexGen, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let format_str = format!(
        "{} {} {} {} {} {} {} {} {} {} {}",
        v.plane_x.normal.x,
        v.plane_x.normal.y,
        v.plane_x.normal.z,
        v.plane_x.distance,
        v.plane_y.normal.x,
        v.plane_y.normal.y,
        v.plane_y.normal.z,
        v.plane_y.distance,
        v.rot,
        v.scale[0],
        v.scale[1]
    );
    format_str.serialize(serializer)
}

fn deserialize_matrix<'de, D>(deserializer: D) -> Result<MatrixF, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match String::deserialize(deserializer) {
        Ok(s) => {
            let coords = s
                .trim()
                .split(' ')
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<f32>>();
            Ok(MatrixF::new(
                coords[0], coords[4], coords[8], coords[12], coords[1], coords[5], coords[9],
                coords[13], coords[2], coords[6], coords[10], coords[14], coords[3], coords[7],
                coords[11], coords[15],
            ))
        }
        Err(e) => Err(e),
    }
}

fn serialize_matrix<S>(v: &MatrixF, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let format_str = format!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        v.x.x,
        v.y.x,
        v.z.x,
        v.w.x,
        v.x.y,
        v.y.y,
        v.z.y,
        v.w.y,
        v.x.z,
        v.y.z,
        v.z.z,
        v.w.z,
        v.x.w,
        v.y.w,
        v.z.w,
        v.w.w
    );
    format_str.serialize(serializer)
}

fn deserialize_propertymap<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Result<HashMap<String, String>, D::Error> =
        serde::Deserialize::deserialize(deserializer);
    match raw {
        Ok(s) => Ok(s
            .iter()
            .map(|(k, v)| (k.strip_prefix('@').unwrap().to_owned(), v.clone()))
            .collect::<HashMap<String, String>>()),
        Err(e) => Err(e),
    }
}

fn serialize_propertymap<S>(v: &HashMap<String, String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let format_str = format!(
        "{{{}}}",
        v.iter()
            .map(|(k, v)| format!("@{}={}", k, v))
            .collect::<Vec<String>>()
            .join(",")
    );
    format_str.serialize(serializer)
}

pub fn preprocess_csx(cscene: &mut ConstructorScene) {
    let mut cur_face_id = 0;
    cscene.detail_levels.detail_level.iter_mut().for_each(|d| {
        d.interior_map.brushes.brush.iter_mut().for_each(|b| {
            b.vertices.vertex.iter_mut().for_each(|v| {
                v.pos = b
                    .transform
                    .transform_point(Point3::from_vec(v.pos))
                    .to_vec();
            });
            b.face.iter_mut().for_each(|f| {
                let mut o = (f.plane.normal * -f.plane.distance).extend(1.0);
                let mut n = f.plane.normal.extend(0.0);
                o = b.transform * o;
                n = b.transform.inverse_transform().unwrap().transpose() * n;
                let norm = n.truncate().normalize();
                let d = -o.truncate().dot(norm);
                f.plane.normal = norm;
                f.plane.distance = d;
                f.face_id = cur_face_id;
                cur_face_id += 1;
            });
        });
    });

    // Fix texgens
    cscene.detail_levels.detail_level.iter_mut().for_each(|d| {
        d.interior_map.brushes.brush.iter_mut().for_each(|b| {
            b.face.iter_mut().for_each(|f| {
                let mut axis_u = f.texgens.plane_x.normal.clone();
                let mut axis_v = f.texgens.plane_y.normal.clone();
                if f.texgens.rot.rem_euclid(360.0) != 0.0 {
                    let up = f.texgens.plane_x.normal.cross(f.texgens.plane_y.normal);
                    let rot_mat = Matrix3::from_axis_angle(up, cgmath::Deg(f.texgens.rot));
                    axis_u = rot_mat * axis_u;
                    axis_v = rot_mat * axis_v;
                }

                // Plane X

                let s1 = (1.0 / f.texgens.scale[0])
                    * (d.interior_map.brush_scale as f32 / f.tex_div[0] as f32);
                let s2 = f.texgens.plane_x.distance / f.tex_div[0] as f32;
                f.texgens.plane_x.normal = axis_u * s1;
                f.texgens.plane_x.distance = s2;

                // Transform the uv axes too

                (f.texgens.plane_x.normal, f.texgens.plane_x.distance) = transform_plane(
                    f.texgens.plane_x.normal,
                    f.texgens.plane_x.distance,
                    b.transform,
                );

                // Plane Y

                let s1 = (1.0 / f.texgens.scale[1])
                    * (d.interior_map.brush_scale as f32 / f.tex_div[1] as f32);
                let s2 = f.texgens.plane_y.distance / f.tex_div[1] as f32;
                f.texgens.plane_y.normal = axis_v * s1;
                f.texgens.plane_y.distance = s2;

                // Transform the uv axes too

                (f.texgens.plane_y.normal, f.texgens.plane_y.distance) = transform_plane(
                    f.texgens.plane_y.normal,
                    f.texgens.plane_y.distance,
                    b.transform,
                );
            });
        });
    });
}

fn transform_plane(
    normal: Vector3<f32>,
    distance: f32,
    transform: Matrix4<f32>,
) -> (Vector3<f32>, f32) {
    let col1len = (transform.x.x * transform.x.x
        + transform.x.y * transform.x.y
        + transform.x.z * transform.x.z)
        .sqrt();
    let col2len = (transform.y.x * transform.y.x
        + transform.y.y * transform.y.y
        + transform.y.z * transform.y.z)
        .sqrt();
    let col3len = (transform.z.x * transform.z.x
        + transform.z.y * transform.z.y
        + transform.z.z * transform.z.z)
        .sqrt();
    let sx = normal.x / col1len;
    let sy = normal.y / col2len;
    let sz = normal.z / col3len;
    let rx =
        transform.x.x * sx / col1len + transform.y.x * sy / col2len + transform.z.x * sz / col3len;
    let ry =
        transform.x.y * sx / col1len + transform.y.y * sy / col2len + transform.z.y * sz / col3len;
    let rz =
        transform.x.z * sx / col1len + transform.y.z * sy / col2len + transform.z.z * sz / col3len;

    let norm = Point3F::new(rx, ry, rz);

    let d = (-distance * (rx * rx + ry * ry + rz * rz) + distance)
        - (((rx * ((rx * -distance) + transform.w.x))
            + (ry * ((ry * -distance) + transform.w.y))
            + (rz * ((rz * -distance) + transform.w.z)))
            + distance)
        + distance;
    return (norm, d);
}

struct MPGroup<'t, 'u> {
    entities: Vec<&'t Entity>,
    triggers: Vec<&'u Entity>,
}

impl MPGroup<'_, '_> {
    pub fn new() -> Self {
        MPGroup {
            entities: vec![],
            triggers: vec![],
        }
    }
}

pub fn convert_csx(
    cscene: &ConstructorScene,
    version: Version,
    mb_only: bool,
    progress_fn: &mut dyn ProgressEventListener,
) -> (Vec<Vec<u8>>, Vec<BSPReport>) {
    // Collect the light entities
    let lights = cscene
        .detail_levels
        .detail_level
        .iter()
        .flat_map(|d| {
            d.interior_map
                .entities
                .entity
                .iter()
                .filter(|e| e.classname.starts_with("light_"))
        })
        .map(|light_ent| Light::new(light_ent))
        .collect::<Vec<_>>();

    let mut detail_levels = cscene
        .detail_levels
        .detail_level
        .iter()
        .enumerate()
        .map(|(i, d)| {
            progress_fn.progress(
                (i + 1) as u32,
                cscene.detail_levels.detail_level.len() as u32,
                "Exporting detail level".to_string(),
                "Exported detail levels".to_string(),
            );
            let face_count: usize = d
                .interior_map
                .brushes
                .brush
                .iter()
                .map(|b| b.face.len())
                .sum();
            let total_splits = (face_count / 16383) + 1;

            let mut split_interiors = vec![];
            let mut cur_builder = DIFBuilder::new(mb_only);
            let mut cur_face_count = 0;
            cur_builder.set_ambient(
                d.interior_map.ambient_color.clone(),
                d.interior_map.ambient_color_emerg.clone(),
            );
            cur_builder.set_lumel_scale(d.interior_map.light_scale);
            cur_builder.set_geometry_scale(d.interior_map.brush_scale);
            cur_builder.set_lights(lights.clone());
            for b in d
                .interior_map
                .brushes
                .brush
                .iter()
                .filter(|b| (b.type_ != 999 && b.type_ != 4) || b.owner == 0)
            {
                let face_count = b.face.len();
                if cur_face_count + face_count > 16383 {
                    progress_fn.progress(
                        (split_interiors.len() + 1) as u32,
                        total_splits as _,
                        "Exporting interior".to_string(),
                        "Exported interiors".to_string(),
                    );
                    split_interiors.push(cur_builder.build(progress_fn));
                    cur_builder = DIFBuilder::new(mb_only);
                    cur_builder.set_ambient(
                        d.interior_map.ambient_color.clone(),
                        d.interior_map.ambient_color_emerg.clone(),
                    );
                    cur_builder.set_lumel_scale(d.interior_map.light_scale);
                    cur_builder.set_geometry_scale(d.interior_map.brush_scale);
                    cur_builder.set_lights(lights.clone());
                    cur_face_count = 0;
                }
                cur_face_count += face_count;
                cur_builder.add_brush(b);
            }
            progress_fn.progress(
                (split_interiors.len() + 1) as u32,
                total_splits as _,
                "Exporting interior".to_string(),
                "Exported interiors".to_string(),
            );
            split_interiors.push(cur_builder.build(progress_fn));
            split_interiors
        })
        .collect::<Vec<_>>();

    let mut reports = vec![];

    let mut dif = dif_with_interiors(
        detail_levels
            .iter_mut()
            .map(|d| {
                let (itr, report) = d.remove(0);
                reports.push(report);
                itr
            })
            .collect_vec(),
    );

    // Do the MPs
    dif.sub_objects = cscene
        .detail_levels
        .detail_level
        .iter()
        .flat_map(|d| {
            let group_query = d
                .interior_map
                .brushes
                .brush
                .iter()
                .filter(|b| b.owner != 0)
                .group_by(|b| b.owner);
            let groups: Vec<_> = group_query.into_iter().collect();
            let group_count = groups.len();
            groups
                .into_iter()
                .sorted_by(|(a, _), (b, _)| a.cmp(b))
                .enumerate()
                .map(|(i, (_, g))| {
                    let mut builder = DIFBuilder::new(mb_only);
                    builder.set_ambient(
                        d.interior_map.ambient_color.clone(),
                        d.interior_map.ambient_color_emerg.clone(),
                    );
                    builder.set_lumel_scale(d.interior_map.light_scale);
                    builder.set_geometry_scale(d.interior_map.brush_scale);
                    builder.set_lights(lights.clone());
                    g.for_each(|b| {
                        builder.add_brush(b);
                    });
                    progress_fn.progress(
                        (i + 1) as u32,
                        group_count as _,
                        "Exporting subobject".to_string(),
                        "Exported subobjects".to_string(),
                    );
                    let (itr, report) = builder.build(progress_fn);
                    reports.push(report);
                    itr
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // path_nodes for MPs, they come after the MP entity
    let path_node_ents = cscene
        .detail_levels
        .detail_level
        .iter()
        .flat_map(|d| {
            d.interior_map.entities.entity.iter().filter(|e| {
                e.classname == "path_node"
                    || e.classname == "Door_Elevator"
                    || e.classname == "trigger"
            })
        })
        .collect::<Vec<_>>();
    if path_node_ents.len() > 0
        && path_node_ents
            .iter()
            .find_position(|e| e.classname == "Door_Elevator")
            .is_some()
    {
        let mut path_node_groups: HashMap<usize, MPGroup> = HashMap::new();
        let mut cur_mp = path_node_ents
            .iter()
            .find_position(|e| e.classname == "Door_Elevator")
            .unwrap()
            .0;
        for (i, &e) in path_node_ents.iter().enumerate() {
            if i < cur_mp {
                continue; // Skip those path_nodes without any Door_Elevator
            }
            if i >= cur_mp && e.classname == "Door_Elevator" {
                cur_mp = i;
                path_node_groups.insert(cur_mp, MPGroup::new());
            }
            if e.classname == "path_node" {
                path_node_groups.get_mut(&cur_mp).unwrap().entities.push(e);
            }
            if e.classname == "trigger" {
                path_node_groups.get_mut(&cur_mp).unwrap().triggers.push(e);
            }
        }

        let mut exported_triggers: Vec<Trigger> = vec![];

        dif.interior_path_followers = path_node_groups
            .iter()
            .enumerate()
            .filter(|(_, (_, v))| v.entities.len() != 0)
            .map(|(i, (&k, v))| {
                let mut props = path_node_ents[k].properties.clone();
                if props.contains_key("datablock") {
                    props.remove("datablock").unwrap();
                }
                InteriorPathFollower {
                    datablock: path_node_ents[k]
                        .properties
                        .get("datablock")
                        .unwrap_or(&"PathedDefault".to_string())
                        .to_owned(),
                    properties: props,
                    name: "MustChange".to_string(),
                    offset: Point3F::new(0.0, 0.0, 0.0),
                    interior_res_index: i as u32,
                    trigger_ids: v
                        .triggers
                        .iter()
                        .map(|t| {
                            let trigger_brushes = cscene
                                .detail_levels
                                .detail_level
                                .iter()
                                .flat_map(|d| {
                                    d.interior_map
                                        .brushes
                                        .brush
                                        .iter()
                                        .filter(|b| b.owner == t.id)
                                        .collect::<Vec<_>>()
                                })
                                .collect::<Vec<_>>();
                            // Just take the last one - there should only be one
                            let trigger_bbox =
                                get_bounding_box_not_owned(trigger_brushes.as_slice());

                            let mut tprops = t.properties.clone();
                            if tprops.contains_key("datablock") {
                                tprops.remove("datablock").unwrap();
                            }

                            let len = exported_triggers.len();
                            exported_triggers.push(build_trigger(
                                t.properties
                                    .get("datablock")
                                    .unwrap_or(&"DefaultTrigger".to_string())
                                    .to_string(),
                                tprops,
                                &trigger_bbox.min,
                                &trigger_bbox.extent(),
                            ));
                            len as u32
                        })
                        .collect::<Vec<_>>(),
                    total_ms: v
                        .entities
                        .iter()
                        .map(|v| {
                            v.properties
                                .get("next_time")
                                .unwrap_or(&"0".to_string())
                                .parse::<u32>()
                                .unwrap_or(0)
                        })
                        .sum(),
                    way_points: v
                        .entities
                        .iter()
                        .map(|v| WayPoint {
                            ms_to_next: v
                                .properties
                                .get("next_time")
                                .unwrap_or(&"0".to_string())
                                .parse::<u32>()
                                .unwrap_or(0),

                            position: v.origin.unwrap(),
                            smoothing_type: v
                                .properties
                                .get("smoothing")
                                .unwrap_or(&"0".to_string())
                                .parse::<u32>()
                                .unwrap_or(0),

                            rotation: QuatF::from_axis_angle(Vector3::new(1.0, 0.0, 0.0), Rad(0.0)),
                        })
                        .collect::<Vec<_>>(),
                }
            })
            .collect::<Vec<_>>();

        dif.triggers = exported_triggers;
    }

    // progress_fn.progress(0, 0, "Exporting entities".to_string(), "Exported entities");
    //  Do the entities
    dif.game_entities = cscene
        .detail_levels
        .detail_level
        .iter()
        .flat_map(|d| {
            d.interior_map
                .entities
                .entity
                .iter()
                .filter(|e| {
                    e.classname != "worldspawn"
                        && e.classname != "Door_Elevator"
                        && e.classname != "path_node"
                        && e.classname != "trigger"
                        && e.properties.contains_key("game_class")
                        && !e.classname.starts_with("light_") // Filter out the light entities
                })
                .map(|e| GameEntity {
                    datablock: e
                        .properties
                        .get("datablock")
                        .unwrap_or(&e.classname)
                        .clone(),
                    position: e.origin.unwrap_or(Vector3::new(0.0, 0.0, 0.0)),
                    game_class: e.properties["game_class"].clone(),
                    properties: e
                        .properties
                        .clone()
                        .into_iter()
                        .filter(|(k, _)| k != "datablock" && k != "game_class")
                        .collect::<HashMap<_, _>>(),
                })
        })
        .collect::<Vec<_>>();

    // The split interiors
    let split_interiors = detail_levels.remove(0);
    let mut split_difs = split_interiors
        .into_iter()
        .map(|(i, _)| dif_with_interiors(vec![i]))
        .collect::<Vec<_>>();

    split_difs.insert(0, dif);

    let dif_data = split_difs
        .into_iter()
        .map(|d| {
            let mut buf = vec![];
            d.write(&mut buf, &version).unwrap();
            buf
        })
        .collect::<Vec<_>>();

    (dif_data, reports)
}

pub fn dif_with_interiors(interiors: Vec<Interior>) -> Dif {
    Dif {
        interiors,
        sub_objects: vec![],
        triggers: vec![],
        interior_path_followers: vec![],
        force_fields: vec![],
        ai_special_nodes: vec![],
        vehicle_collision: None,
        game_entities: vec![],
    }
}

fn build_trigger(
    datablock: String,
    properties: Dictionary,
    pos: &Point3F,
    size: &Point3F,
) -> Trigger {
    Trigger {
        name: "MustChange".to_string(),
        datablock: datablock,
        offset: Point3F::new(0.0, 0.0, 0.0),
        properties: properties,
        polyhedron: Polyhedron {
            point_list: vec![
                Point3F::new(pos.x, pos.y, pos.z + size.z),
                Point3F::new(pos.x, pos.y + size.y, pos.z + size.z),
                Point3F::new(pos.x + size.x, pos.y + size.y, pos.z + size.z),
                Point3F::new(pos.x + size.x, pos.y, pos.z + size.z),
                Point3F::new(pos.x, pos.y, pos.z),
                Point3F::new(pos.x, pos.y + size.y, pos.z),
                Point3F::new(pos.x + size.x, pos.y + size.y, pos.z),
                Point3F::new(pos.x + size.x, pos.y, pos.z),
            ],
            plane_list: vec![
                PlaneF {
                    normal: Point3F::new(-1.0, 0.0, 0.0),
                    distance: pos.x,
                },
                PlaneF {
                    normal: Point3F::new(0.0, 1.0, 0.0),
                    distance: pos.y + size.y,
                },
                PlaneF {
                    normal: Point3F::new(1.0, 0.0, 0.0),
                    distance: pos.x + size.x,
                },
                PlaneF {
                    normal: Point3F::new(0.0, -1.0, 0.0),
                    distance: pos.y,
                },
                PlaneF {
                    normal: Point3F::new(0.0, 0.0, 1.0),
                    distance: pos.z + size.z,
                },
                PlaneF {
                    normal: Point3F::new(0.0, 0.0, -1.0),
                    distance: pos.z,
                },
            ],
            edge_list: vec![
                PolyhedronEdge {
                    face0: 0,
                    face1: 4,
                    vertex0: 0,
                    vertex1: 1,
                },
                PolyhedronEdge {
                    face0: 5,
                    face1: 0,
                    vertex0: 4,
                    vertex1: 5,
                },
                PolyhedronEdge {
                    face0: 3,
                    face1: 0,
                    vertex0: 0,
                    vertex1: 4,
                },
                PolyhedronEdge {
                    face0: 1,
                    face1: 4,
                    vertex0: 1,
                    vertex1: 2,
                },
                PolyhedronEdge {
                    face0: 5,
                    face1: 6,
                    vertex0: 5,
                    vertex1: 1,
                },
                PolyhedronEdge {
                    face0: 0,
                    face1: 1,
                    vertex0: 1,
                    vertex1: 5,
                },
                PolyhedronEdge {
                    face0: 2,
                    face1: 4,
                    vertex0: 2,
                    vertex1: 3,
                },
                PolyhedronEdge {
                    face0: 5,
                    face1: 2,
                    vertex0: 6,
                    vertex1: 7,
                },
                PolyhedronEdge {
                    face0: 1,
                    face1: 2,
                    vertex0: 2,
                    vertex1: 6,
                },
                PolyhedronEdge {
                    face0: 3,
                    face1: 4,
                    vertex0: 3,
                    vertex1: 0,
                },
                PolyhedronEdge {
                    face0: 5,
                    face1: 3,
                    vertex0: 7,
                    vertex1: 4,
                },
                PolyhedronEdge {
                    face0: 2,
                    face1: 3,
                    vertex0: 3,
                    vertex1: 7,
                },
            ],
        },
    }
}
