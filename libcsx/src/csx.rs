use std::collections::HashMap;

use cgmath::{EuclideanSpace, InnerSpace, Matrix, Matrix3, Matrix4, Point3, Transform, Vector3};
use dif::{
    dif::Dif,
    game_entity::GameEntity,
    interior::Interior,
    io::{Version, Writable},
    types::{MatrixF, PlaneF, Point3F},
};
use itertools::Itertools;
use serde::Deserialize;

use crate::builder::{BSPReport, DIFBuilder, ProgressEventListener};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConstructorScene {
    #[serde(rename = "DetailLevels")]
    pub detail_levels: DetailLevels,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DetailLevels {
    pub detail_level: Vec<DetailLevel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DetailLevel {
    pub interior_map: InteriorMap,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InteriorMap {
    pub entities: Entities,
    pub brushes: Brushes,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Entities {
    pub entity: Vec<Entity>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Entity {
    #[serde(rename = "@id")]
    pub id: i32,
    #[serde(rename = "@classname")]
    pub classname: String,
    #[serde(rename = "@gametype")]
    pub gametype: String,
    #[serde(default)]
    #[serde(rename = "@origin", deserialize_with = "deserialize_point_optional")]
    pub origin: Option<Point3F>,
    #[serde(deserialize_with = "deserialize_propertymap")]
    pub properties: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EntityProperties {
    #[serde(rename = "$value", deserialize_with = "deserialize_propertymap")]
    pub property_map: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Brushes {
    pub brush: Vec<Brush>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Brush {
    #[serde(rename = "@id")]
    pub id: i32,
    #[serde(rename = "@owner")]
    pub owner: i32,
    #[serde(rename = "@type")]
    pub type_: i32,
    #[serde(rename = "@transform", deserialize_with = "deserialize_matrix")]
    pub transform: MatrixF,
    pub vertices: Vertices,
    pub face: Vec<Face>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Vertices {
    pub vertex: Vec<Vertex>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Vertex {
    #[serde(rename = "@pos", deserialize_with = "deserialize_point")]
    pub pos: Point3F,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Face {
    #[serde(rename = "@id")]
    pub id: i32,
    #[serde(rename = "@plane", deserialize_with = "deserialize_plane")]
    pub plane: PlaneF,
    #[serde(rename = "@material")]
    pub material: String,
    #[serde(rename = "@texgens", deserialize_with = "deserialize_texgen")]
    pub texgens: TexGen,
    #[serde(rename = "@texDiv", deserialize_with = "deserialize_number_list")]
    pub tex_div: Vec<i32>,
    pub indices: Indices,
    #[serde(skip_deserializing)]
    pub face_id: i32,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Indices {
    #[serde(rename = "@indices", deserialize_with = "deserialize_number_list")]
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

                let s1 = (1.0 / f.texgens.scale[0]) * (32.0 / f.tex_div[0] as f32);
                let s2 = f.texgens.plane_x.distance / f.tex_div[0] as f32;
                f.texgens.plane_x.normal = axis_u * s1;
                f.texgens.plane_x.distance = s2;

                // Transform the uv axes too

                (f.texgens.plane_x.normal, f.texgens.plane_x.distance) = transform_plane(
                    axis_u,
                    f.texgens.plane_x.normal,
                    f.texgens.plane_x.distance,
                    s1,
                    b.transform,
                );

                // Plane Y

                let s1 = (-1.0 / f.texgens.scale[1]) * (32.0 / f.tex_div[1] as f32);
                let s2 = f.texgens.plane_y.distance / f.tex_div[1] as f32;
                f.texgens.plane_y.normal = axis_v * -s1;
                f.texgens.plane_y.distance = s2;

                // Transform the uv axes too

                (f.texgens.plane_y.normal, f.texgens.plane_y.distance) = transform_plane(
                    axis_v,
                    f.texgens.plane_y.normal,
                    f.texgens.plane_y.distance,
                    -s1,
                    b.transform,
                );
            });
        });
    });
}

fn transform_plane(
    axis: cgmath::Vector3<f32>,
    normal: Vector3<f32>,
    distance: f32,
    s: f32,
    transform: Matrix4<f32>,
) -> (Vector3<f32>, f32) {
    let mut o = (axis * distance * (1.0 / -s)).extend(1.0);
    let mut n = normal.extend(0.0);
    o = transform * o;
    n = transform.inverse_transform().unwrap().transpose() * n;
    let norm = n.truncate();
    let d = -o.truncate().dot(norm);
    return (norm, d);
}

pub fn convert_csx(
    cscene: &ConstructorScene,
    version: Version,
    mb_only: bool,
    progress_fn: &mut dyn ProgressEventListener,
) -> (Vec<Vec<u8>>, Vec<BSPReport>) {
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
            for b in d
                .interior_map
                .brushes
                .brush
                .iter()
                .filter(|b| b.type_ != 999 || b.owner == 0)
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
                .enumerate()
                .map(|(i, (_, g))| {
                    let mut builder = DIFBuilder::new(mb_only);
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
                        && e.properties.contains_key("game_class")
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

fn dif_with_interiors(interiors: Vec<Interior>) -> Dif {
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
