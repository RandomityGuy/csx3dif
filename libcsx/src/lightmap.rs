#![forbid(unsafe_code)]

use cgmath::{InnerSpace, Vector4};
use dif::{
    interior::{BSPIndex, Interior, Surface},
    types::{Point2F, Point3F},
};
use rayon::prelude::*;

use crate::{builder::RaycastCalc, light::Light};

/// A rectangle defined by position and size.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    /// Position of the rectangle.
    pub position: Point2F,
    /// Size of the rectangle, where X - width, Y - height.
    pub size: Point2F,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            position: Point2F::new(0.0, 0.0),
            size: Point2F::new(0.0, 0.0),
        }
    }
}

impl Rect {
    /// Creates a new rectangle from X, Y, width, height.
    #[inline]
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            position: Point2F::new(x, y),
            size: Point2F::new(w, h),
        }
    }

    /// Checks if the rectangle intersects with some other rectangle.
    #[inline]
    pub fn intersects(&self, other: Rect) -> bool {
        if self.position.x < other.position.x + other.size.x
            && other.position.x < self.position.x + self.size.x
            && self.position.y < other.position.y + other.size.y
            && other.position.y < self.position.y + self.size.y
        {
            return true;
        }
        // Contains
        if self.position.x <= other.position.x
            && self.position.y <= other.position.y
            && self.position.x + self.size.x >= other.position.x + other.size.x
            && self.position.y + self.size.y >= other.position.y + other.size.y
        {
            return true;
        }
        if other.position.x <= self.position.x
            && other.position.y <= self.position.y
            && other.position.x + other.size.x >= self.position.x + self.size.x
            && other.position.y + other.size.y >= self.position.y + self.size.y
        {
            return true;
        }

        false
    }
}

pub struct LightmapSurface {
    pub surface_index: usize,
    pub sc: Point3F,
    pub tc: Point3F,
    pub dx: f32,
    pub dy: f32,
    pub offset_x: usize,
    pub offset_y: usize,
    pub width: usize,
    pub height: usize,
    pub normal: Point3F,
    pub tri_points: Vec<Point3F>,
    pub lightmap_index: usize,
}

#[inline]
pub fn get_barycentric_coords_2d(
    p: Point2F,
    a: Point2F,
    b: Point2F,
    c: Point2F,
) -> (f32, f32, f32) {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;

    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let inv_denom = 1.0 / (d00 * d11 - d01.powi(2));

    let v = (d11 * d20 - d01 * d21) * inv_denom;
    let w = (d00 * d21 - d01 * d20) * inv_denom;
    let u = 1.0 - v - w;

    (u, v, w)
}

#[inline]
pub fn barycentric_to_world(
    bary: (f32, f32, f32),
    pa: &Point3F,
    pb: &Point3F,
    pc: &Point3F,
) -> Point3F {
    pa * bary.0 + pb * bary.1 + pc * bary.2
}

#[inline]
pub fn barycentric_is_inside(bary: (f32, f32, f32)) -> bool {
    (bary.0 >= 0.0) && (bary.1 >= 0.0) && (bary.0 + bary.1 < 1.0)
}

// Calculates properties of pixel (world position, normal) at given position.
fn pick(
    uv: Point2F,
    grid: &Grid,
    data: &[LightmapSurface],
    lumel_scale: u32,
) -> Option<(Point3F, Point3F)> {
    if let Some(cell) = grid.pick(uv) {
        for surf in cell.triangles.iter().map(|surf_idx| &data[*surf_idx]) {
            // let (si, ti, axis) = if surf.sc[0] == 0.0 && surf.tc[0] == 0.0 {
            //     if surf.sc[1] == 0.0 {
            //         (2, 1, 0)
            //     } else {
            //         (1, 2, 0)
            //     }
            // } else if surf.sc[1] == 0.0 && surf.tc[1] == 0.0 {
            //     if surf.sc[0] == 0.0 {
            //         (2, 0, 1)
            //     } else {
            //         (0, 2, 1)
            //     }
            // } else if surf.sc[2] == 0.0 && surf.tc[2] == 0.0 {
            //     if surf.sc[0] == 0.0 {
            //         (1, 0, 2)
            //     } else {
            //         (0, 1, 2)
            //     }
            // } else {
            //     panic!("Bad texgens for lightmap!")
            // };

            // let plane_dist = -surf.normal.dot(surf.tri_points[0]);

            // let mut start = Point3F::new(0.0, 0.0, 0.0);
            // start[si] = -surf.dx * lumel_scale as f32;
            // start[ti] = -surf.dy * lumel_scale as f32;
            // start[axis] =
            //     (surf.normal[si] * start[si]) + (surf.normal[ti] * start[ti]) + plane_dist;

            // let mut s_vec = Point3F::new(0.0, 0.0, 0.0);
            // let mut t_vec = Point3F::new(0.0, 0.0, 0.0);
            // s_vec[si] = 1.0;
            // s_vec[ti] = 0.0;
            // t_vec[ti] = 1.0;
            // t_vec[si] = 0.0;

            // let mut plane_normal = surf.normal.clone();
            // plane_normal[ti] = 0.0;
            // plane_normal = plane_normal.normalize();

            // let angle = plane_normal[axis].clamp(-1.0, 1.0).acos();
            // s_vec[axis] = if plane_normal[si] < 0.0 {
            //     (-angle).tan()
            // } else {
            //     angle.tan()
            // };

            // let mut plane_normal = surf.normal.clone();
            // plane_normal[si] = 0.0;
            // plane_normal = plane_normal.normalize();

            // let angle = plane_normal[axis].clamp(-1.0, 1.0).acos();
            // t_vec[axis] = if plane_normal[ti] < 0.0 {
            //     (-angle).tan()
            // } else {
            //     angle.tan()
            // };

            // s_vec *= lumel_scale as f32;
            // t_vec *= lumel_scale as f32;

            // let uv_off_x = ((uv.x - 0.5 / grid.atlas_size as f32) / grid.atlas_size as f32)
            //     - surf.offset_x as f32;
            // let uv_off_y = ((uv.y - 0.5 / grid.atlas_size as f32) / grid.atlas_size as f32)
            //     - surf.offset_y as f32;

            // return Some((s_vec * uv_off_x + t_vec * uv_off_y + start, surf.normal));

            let mut i = 0;
            while i < surf.tri_points.len() {
                let p1 = &surf.tri_points[i];
                let p2 = &surf.tri_points[i + 1];
                let p3 = &surf.tri_points[i + 2];
                let uv1 = Point2F::new(p1.dot(surf.sc) + surf.dx, p1.dot(surf.tc) + surf.dy);
                let uv2 = Point2F::new(p2.dot(surf.sc) + surf.dx, p2.dot(surf.tc) + surf.dy);
                let uv3 = Point2F::new(p3.dot(surf.sc) + surf.dx, p3.dot(surf.tc) + surf.dy);

                let center = (uv1 + uv2 + uv3) / 3.0;
                let to_center = (center - uv).normalize() / 3.0;

                let mut current_uv = uv;
                for _ in 0..3 {
                    let barycentric = get_barycentric_coords_2d(current_uv, uv1, uv2, uv3);

                    if barycentric_is_inside(barycentric) {
                        return Some((barycentric_to_world(barycentric, p1, p2, p3), surf.normal));
                    }

                    // Offset uv to center for conservative rasterization.
                    current_uv += to_center;
                }

                i += 3;
            }
            return None;
        }
    }
    None
}

struct GridCell {
    // List of triangle indices.
    triangles: Vec<usize>,
}

struct Grid {
    cells: Vec<GridCell>,
    size: usize,
    fsize: f32,
    atlas_size: usize,
}

impl Grid {
    // Creates uniform grid where each cell contains list of triangles whose second texture
    // coordinates intersects with it.
    fn new(data: &[LightmapSurface], atlas_size: usize, size: usize, lmap_index: usize) -> Self {
        let mut cells = Vec::with_capacity(size);
        let fsize = size as f32;
        let atlas_fsize = atlas_size as f32;
        for y in 0..size {
            for x in 0..size {
                let bounds =
                    Rect::new(x as f32 / fsize, y as f32 / fsize, 1.0 / fsize, 1.0 / fsize);

                let mut triangles = Vec::new();

                for (idx, surf) in data.iter().enumerate() {
                    if surf.lightmap_index != lmap_index {
                        continue;
                    }
                    let mut i = 0;

                    let surface_bounds = Rect::new(
                        surf.offset_x as f32 / atlas_fsize,
                        surf.offset_y as f32 / atlas_fsize,
                        surf.width as f32 / atlas_fsize,
                        surf.height as f32 / atlas_fsize,
                    );
                    assert!(
                        surf.width < atlas_size,
                        "Lightmap surface too big: {}x{}",
                        surf.width,
                        surf.height
                    );
                    assert!(
                        surf.height < atlas_size,
                        "Lightmap surface too big: {}x{}",
                        surf.width,
                        surf.height
                    );
                    assert!(
                        surf.offset_x < atlas_size,
                        "Lightmap surface too big: {}x{}",
                        surf.width,
                        surf.height
                    );
                    assert!(
                        surf.offset_y < atlas_size,
                        "Lightmap surface too big: {}x{}",
                        surf.width,
                        surf.height
                    );

                    if surface_bounds.intersects(bounds) {
                        triangles.push(idx);
                        // while i < surf.tri_points.len() {
                        //     triangles.push((idx, i));

                        //     i += 3;
                        // }
                    }

                    // while i < surf.tri_points.len() {
                    //     let p1 = &surf.tri_points[i];
                    //     let p2 = &surf.tri_points[i + 1];
                    //     let p3 = &surf.tri_points[i + 2];
                    //     let uv1 =
                    //         Point2F::new(p1.dot(surf.sc) + surf.dx, p1.dot(surf.tc) + surf.dy);
                    //     let uv2 =
                    //         Point2F::new(p2.dot(surf.sc) + surf.dx, p2.dot(surf.tc) + surf.dy);
                    //     let uv3 =
                    //         Point2F::new(p3.dot(surf.sc) + surf.dx, p3.dot(surf.tc) + surf.dy);

                    //     let uv_min =
                    //         Point2F::new(uv1.x.min(uv2.x).min(uv3.x), uv1.y.min(uv2.y).min(uv3.y));
                    //     let uv_max =
                    //         Point2F::new(uv1.x.max(uv2.x).max(uv3.x), uv1.y.max(uv2.y).max(uv3.y));
                    //     let triangle_bounds =
                    //         Rect::new(uv_min.x, uv_min.y, uv_max.x - uv_min.x, uv_max.y - uv_min.y);
                    //     if triangle_bounds.intersects(bounds) {
                    //         triangles.push((idx, i));
                    //     }

                    //     i += 3;
                    // }
                }

                cells.push(GridCell { triangles })
            }
        }

        Self {
            cells,
            size,
            fsize: size as f32,
            atlas_size,
        }
    }

    fn pick(&self, v: Point2F) -> Option<&GridCell> {
        let ix = (v.x * self.fsize) as usize;
        let iy = (v.y * self.fsize) as usize;
        self.cells.get(iy * self.size + ix)
    }
}

#[derive(Clone, Debug)]
pub struct LightMap {
    pub pixels: Vec<u8>,
}

impl LightMap {
    pub fn new(
        interior: &Interior,
        surfaces: &[LightmapSurface],
        lights: &[Light],
        atlas_size: u32,
        lmap_index: usize,
        lumel_scale: u32,
    ) -> Self {
        // We have to re-generate new set of world-space vertices because UV generator
        // may add new vertices on seams.
        let scale = 1.0 / atlas_size as f32;
        let half_pixel = scale * 0.5;
        let grid = Grid::new(
            surfaces,
            atlas_size as usize,
            (atlas_size as usize / 32).max(4),
            lmap_index,
        );

        let mut pixels: Vec<Vector4<u8>> =
            vec![Vector4::new(0, 0, 0, 0); atlas_size as usize * atlas_size as usize];

        // Color the used pixels pink pls, for debug
        // for surf in surfaces.iter() {
        //     if surf.lightmap_index != lmap_index {
        //         continue;
        //     }
        //     let start_x = surf.offset_x;
        //     let start_y = surf.offset_y;
        //     let end_x = surf.offset_x + surf.width;
        //     let end_y = surf.offset_y + surf.height;
        //     for y in start_y..end_y {
        //         for x in start_x..end_x {
        //             pixels[y * atlas_size as usize + x] = Vector4::new(255, 0, 255, 255);
        //         }
        //     }
        // }

        // Actually the lightmap process, light each surface
        for surf in surfaces.iter() {
            if surf.lightmap_index != lmap_index {
                continue;
            }

            let (si, ti, axis) = if surf.sc[0] == 0.0 && surf.tc[0] == 0.0 {
                if surf.sc[1] == 0.0 {
                    (2, 1, 0)
                } else {
                    (1, 2, 0)
                }
            } else if surf.sc[1] == 0.0 && surf.tc[1] == 0.0 {
                if surf.sc[0] == 0.0 {
                    (2, 0, 1)
                } else {
                    (0, 2, 1)
                }
            } else if surf.sc[2] == 0.0 && surf.tc[2] == 0.0 {
                if surf.sc[0] == 0.0 {
                    (1, 0, 2)
                } else {
                    (0, 1, 2)
                }
            } else {
                panic!("Bad texgens for lightmap!")
            };

            let plane_dist = -surf.normal.dot(surf.tri_points[0]);

            let mut start = Point3F::new(0.0, 0.0, 0.0);
            start[si] = -surf.dx * lumel_scale as f32;
            start[ti] = -surf.dy * lumel_scale as f32;
            start[axis] =
                (surf.normal[si] * start[si]) + (surf.normal[ti] * start[ti]) + plane_dist;

            let mut s_vec = Point3F::new(0.0, 0.0, 0.0);
            let mut t_vec = Point3F::new(0.0, 0.0, 0.0);
            s_vec[si] = 1.0;
            s_vec[ti] = 0.0;
            t_vec[ti] = 1.0;
            t_vec[si] = 0.0;

            let mut plane_normal = surf.normal.clone();
            plane_normal[ti] = 0.0;
            plane_normal = plane_normal.normalize();

            let angle = plane_normal[axis].clamp(-1.0, 1.0).acos();
            s_vec[axis] = if plane_normal[si] < 0.0 {
                (-angle).tan()
            } else {
                angle.tan()
            };

            let mut plane_normal = surf.normal.clone();
            plane_normal[si] = 0.0;
            plane_normal = plane_normal.normalize();

            let angle = plane_normal[axis].clamp(-1.0, 1.0).acos();
            t_vec[axis] = if plane_normal[ti] < 0.0 {
                (-angle).tan()
            } else {
                angle.tan()
            };

            s_vec *= lumel_scale as f32;
            t_vec *= lumel_scale as f32;

            let s_run = s_vec * surf.width as f32;

            let mut world_position = start;

            let s_run = s_vec * surf.width as f32;

            let mut world_position = surf.tri_points[0];

            let start_x = surf.offset_x;
            let start_y = surf.offset_y;
            let end_x = surf.offset_x + surf.width;
            let end_y = surf.offset_y + surf.height;
            for y in start_y..end_y {
                for x in start_x..end_x {
                    //let uv =
                    //    Point2F::new(x as f32 * scale + half_pixel, y as f32 * scale + half_pixel);

                    let mut pixel_color = Point3F::new(0.0, 0.0, 0.0);

                    // let mut i = 0;
                    // 'outer: while i < surf.tri_points.len() {
                    //     let p1 = &surf.tri_points[i];
                    //     let p2 = &surf.tri_points[i + 1];
                    //     let p3 = &surf.tri_points[i + 2];
                    //     let uv1 =
                    //         Point2F::new(p1.dot(surf.sc) + surf.dx, p1.dot(surf.tc) + surf.dy);
                    //     let uv2 =
                    //         Point2F::new(p2.dot(surf.sc) + surf.dx, p2.dot(surf.tc) + surf.dy);
                    //     let uv3 =
                    //         Point2F::new(p3.dot(surf.sc) + surf.dx, p3.dot(surf.tc) + surf.dy);

                    //     let center = (uv1 + uv2 + uv3) / 3.0;
                    //     let to_center = (center - uv).normalize() / 3.0;

                    //     let mut current_uv = uv;
                    //     for _ in 0..3 {
                    //         let barycentric = get_barycentric_coords_2d(current_uv, uv1, uv2, uv3);

                    //         if barycentric_is_inside(barycentric) {
                    //             let world_position = barycentric_to_world(barycentric, p1, p2, p3);
                    for light in lights {
                        let mut attenuation = light.calculate_intensity(&world_position);
                        let light_color = light.get_base_color();
                        // Shadows
                        if attenuation >= 0.01 {
                            let pidx = u16::MAX;
                            let start_node_index = BSPIndex {
                                index: 0,
                                leaf: false,
                                solid: false,
                            };

                            let light_pos = light.get_position();
                            let dir = (light_pos - world_position).normalize();
                            let end = world_position - dir * 0.1;

                            if interior.bsp_ray_cast(&start_node_index, &pidx, light_pos, end) {
                                attenuation = 0.0;
                            }
                        }
                        pixel_color += light_color * attenuation;
                        //     }
                        //     break 'outer;
                    }

                    // Offset uv to center for conservative rasterization.
                    // current_uv += to_center;
                    // }

                    // i += 3;
                    // }

                    pixels[y * atlas_size as usize + x] = Vector4::new(
                        (pixel_color.x.clamp(0.0, 1.0) * 255.0) as u8,
                        (pixel_color.y.clamp(0.0, 1.0) * 255.0) as u8,
                        (pixel_color.z.clamp(0.0, 1.0) * 255.0) as u8,
                        255, // Indicates that this pixel was "filled"
                    );

                    world_position += s_vec;
                }
                world_position -= s_run;
                world_position += t_vec;
            }
        }

        // pixels
        //     .iter_mut()
        //     .enumerate()
        //     .for_each(|(i, pixel): (usize, &mut Vector4<u8>)| {
        //         let x = i % atlas_size as usize;
        //         let y = i / atlas_size as usize;

        //         let uv = Point2F::new(x as f32 * scale + half_pixel, y as f32 * scale + half_pixel);

        //         if let Some((world_position, world_normal)) = pick(uv, &grid, surfaces, lumel_scale)
        //         {
        //             let mut pixel_color = Point3F::new(0.0, 0.0, 0.0);
        //             for light in lights {
        //                 let mut attenuation = light.calculate_intensity(&world_position);
        //                 let light_color = light.get_base_color();
        //                 // Shadows
        //                 if attenuation >= 0.01 {
        //                     // let mut query_buffer = ArrayVec::<usize, 64>::new();
        //                     // let shadow_bias = 0.01;
        //                     // let ray = Ray::from_two_points(light_position, world_position);
        //                     // 'outer_loop: for other_instance in other_meshes {
        //                     //     other_instance
        //                     //         .octree
        //                     //         .ray_query_static(&ray, &mut query_buffer);
        //                     //     for &node in query_buffer.iter() {
        //                     //         match other_instance.octree.node(node) {
        //                     //             OctreeNode::Leaf { indices, .. } => {
        //                     //                 let other_data = other_instance;
        //                     //                 for &triangle_index in indices {
        //                     //                     let triangle =
        //                     //                         &other_data.triangles[triangle_index as usize];
        //                     //                     let va = other_data.vertices[triangle[0] as usize]
        //                     //                         .world_position;
        //                     //                     let vb = other_data.vertices[triangle[1] as usize]
        //                     //                         .world_position;
        //                     //                     let vc = other_data.vertices[triangle[2] as usize]
        //                     //                         .world_position;
        //                     //                     if let Some(pt) =
        //                     //                         ray.triangle_intersection_point(&[va, vb, vc])
        //                     //                     {
        //                     //                         if ray.origin.metric_distance(&pt) + shadow_bias
        //                     //                             < ray.dir.norm()
        //                     //                         {
        //                     //                             attenuation = 0.0;
        //                     //                             break 'outer_loop;
        //                     //                         }
        //                     //                     }
        //                     //                 }
        //                     //             }
        //                     //             OctreeNode::Branch { .. } => unreachable!(),
        //                     //         }
        //                     //     }
        //                     // }
        //                 }
        //                 pixel_color += light_color * attenuation;
        //             }

        //             *pixel = Vector4::new(
        //                 (pixel_color.x.clamp(0.0, 1.0) * 255.0) as u8,
        //                 (pixel_color.y.clamp(0.0, 1.0) * 255.0) as u8,
        //                 (pixel_color.z.clamp(0.0, 1.0) * 255.0) as u8,
        //                 255, // Indicates that this pixel was "filled"
        //             );
        //         }
        //     });

        // Prepare light map for bilinear filtration. This step is mandatory to prevent bleeding.
        let mut rgb_pixels: Vec<Point3F> = Vec::with_capacity((atlas_size * atlas_size) as usize);
        for y in 0..(atlas_size as i32) {
            for x in 0..(atlas_size as i32) {
                let fetch = |dx: i32, dy: i32| -> Option<Point3F> {
                    pixels
                        .get(((y + dy) * (atlas_size as i32) + x + dx) as usize)
                        .and_then(|p| {
                            if p.w != 0 {
                                Some(Point3F::new(p.x as f32, p.y as f32, p.z as f32))
                            } else {
                                None
                            }
                        })
                };

                let src_pixel = pixels[(y * (atlas_size as i32) + x) as usize];
                if src_pixel.w == 0 {
                    // Check neighbour pixels marked as "filled" and use it as value.
                    if let Some(west) = fetch(-1, 0) {
                        rgb_pixels.push(west);
                    } else if let Some(east) = fetch(1, 0) {
                        rgb_pixels.push(east);
                    } else if let Some(north) = fetch(0, -1) {
                        rgb_pixels.push(north);
                    } else if let Some(south) = fetch(0, 1) {
                        rgb_pixels.push(south);
                    } else if let Some(north_west) = fetch(-1, -1) {
                        rgb_pixels.push(north_west);
                    } else if let Some(north_east) = fetch(1, -1) {
                        rgb_pixels.push(north_east);
                    } else if let Some(south_east) = fetch(1, 1) {
                        rgb_pixels.push(south_east);
                    } else if let Some(south_west) = fetch(-1, 1) {
                        rgb_pixels.push(south_west);
                    } else {
                        rgb_pixels.push(Point3F::new(0.0, 0.0, 0.0));
                    }
                } else {
                    rgb_pixels.push(Point3F::new(
                        src_pixel.x as f32,
                        src_pixel.y as f32,
                        src_pixel.z as f32,
                    ))
                }
            }
        }

        // Blur lightmap using simplest box filter.
        let mut bytes = Vec::with_capacity((atlas_size * atlas_size * 3) as usize);
        for y in 0..(atlas_size as i32) {
            for x in 0..(atlas_size as i32) {
                if x < 1 || y < 1 || x + 1 == atlas_size as i32 || y + 1 == atlas_size as i32 {
                    bytes.push(rgb_pixels[(y * (atlas_size as i32) + x) as usize].x as u8);
                    bytes.push(rgb_pixels[(y * (atlas_size as i32) + x) as usize].y as u8);
                    bytes.push(rgb_pixels[(y * (atlas_size as i32) + x) as usize].z as u8);
                } else {
                    let fetch = |dx: i32, dy: i32| -> Point3F {
                        let u8_pixel =
                            rgb_pixels[((y + dy) * (atlas_size as i32) + x + dx) as usize];
                        Point3F::new(u8_pixel.x as f32, u8_pixel.y as f32, u8_pixel.z as f32)
                    };

                    let north_west = fetch(-1, -1);
                    let north = fetch(0, -1);
                    let north_east = fetch(1, -1);
                    let west = fetch(-1, 0);
                    let center = fetch(0, 0);
                    let east = fetch(1, 0);
                    let south_west = fetch(-1, 1);
                    let south = fetch(0, 1);
                    let south_east = fetch(-1, 1);

                    let sum = north_west
                        + north
                        + north_east
                        + west
                        + center
                        + east
                        + south_west
                        + south
                        + south_east;

                    bytes.push((sum.x / 9.0).clamp(0.0, 255.0) as u8);
                    bytes.push((sum.y / 9.0).clamp(0.0, 255.0) as u8);
                    bytes.push((sum.z / 9.0).clamp(0.0, 255.0) as u8);
                }
            }
        }

        Self { pixels: bytes }
    }
}
