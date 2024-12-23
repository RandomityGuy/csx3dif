use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;

use crate::bsp::build_bsp;
use crate::bsp::CSXBSPNode;
use crate::csx::Brush;
use crate::csx::Face;
use crate::csx::TexGen;
use crate::csx::Vertex;
use crate::light::Light;
use crate::lightmap;
use crate::lightmap::LightmapSurface;
use cgmath::AbsDiffEq;
use cgmath::InnerSpace;
use cgmath::Transform;
use cgmath::Vector3;
use dif::interior::*;
use dif::types::*;
use image::codecs::png::PngEncoder;
use image::ImageBuffer;
use image::ImageEncoder;
use image::Rgb;
use rectangle_pack::contains_smallest_box;
use rectangle_pack::pack_rects;
use rectangle_pack::volume_heuristic;
use rectangle_pack::GroupedRectsToPlace;
use rectangle_pack::RectToInsert;
use rectangle_pack::TargetBin;
use std::hash::Hash;

pub trait ProgressEventListener {
    fn progress(&mut self, current: u32, total: u32, status: String, finish_status: String);
}

pub struct BSPReport {
    pub balance_factor: i32,
    pub hit: i32,
    pub total: usize,
    pub hit_area_percentage: f32,
}

pub struct DIFBuilder {
    brushes: Vec<Brush>,
    interior: Interior,
    face_to_surface: HashMap<i32, SurfaceIndex>,
    face_to_plane: HashMap<i32, PlaneIndex>,
    plane_map: HashMap<OrdPlaneF, PlaneIndex>,
    point_map: HashMap<OrdPoint, PointIndex>,
    normal_map: HashMap<OrdPoint, NormalIndex>,
    texgen_map: HashMap<OrdTexGen, TexGenIndex>,
    emit_string_map: HashMap<Vec<u8>, EmitStringIndex>,
    mb_only: bool,
    bsp_report: BSPReport,
    ambient_color: Point3F,
    emergency_ambient_color: Point3F,
    lumel_scale: u32,
    geometry_scale: u32,
    lights: Vec<Light>,
}

pub static mut POINT_EPSILON: f32 = 1e-6;
pub static mut PLANE_EPSILON: f32 = 1e-5;

impl DIFBuilder {
    pub fn new(mb_only: bool) -> DIFBuilder {
        return DIFBuilder {
            brushes: vec![],
            interior: empty_interior(),
            face_to_surface: HashMap::new(),
            face_to_plane: HashMap::new(),
            plane_map: HashMap::new(),
            point_map: HashMap::new(),
            normal_map: HashMap::new(),
            texgen_map: HashMap::new(),
            emit_string_map: HashMap::new(),
            mb_only: mb_only,
            bsp_report: BSPReport {
                balance_factor: 0,
                hit: 0,
                total: 0,
                hit_area_percentage: 0.0,
            },
            ambient_color: Point3F::new(0.0, 0.0, 0.0),
            emergency_ambient_color: Point3F::new(0.0, 0.0, 0.0),
            lumel_scale: 8,
            geometry_scale: 32,
            lights: vec![],
        };
    }

    pub fn add_brush(&mut self, brush: &Brush) {
        self.brushes.push(brush.clone());
    }

    pub fn set_ambient(&mut self, ambient: Point3F, emergency_ambient: Point3F) {
        self.ambient_color = ambient;
        self.emergency_ambient_color = emergency_ambient;
    }

    pub fn set_lumel_scale(&mut self, scale: u32) {
        self.lumel_scale = scale;
    }

    pub fn set_geometry_scale(&mut self, scale: u32) {
        self.geometry_scale = scale;
    }

    pub fn set_lights(&mut self, lights: Vec<Light>) {
        self.lights = lights;
    }

    pub fn build(
        mut self,
        progress_report_callback: &mut dyn ProgressEventListener,
    ) -> (Interior, BSPReport) {
        self.interior.bounding_box = get_bounding_box(&self.brushes);
        self.interior.bounding_sphere = get_bounding_sphere(&self.brushes);
        self.export_brushes(progress_report_callback);
        self.interior.zones.push(Zone {
            portal_start: PortalIndex::new(0),
            portal_count: 0,
            surface_start: 0,
            surface_count: self.interior.surfaces.len() as _,
            static_mesh_start: StaticMeshIndex::new(0),
            static_mesh_count: 0,
            flags: 0,
        });
        self.export_coord_bins();
        if self.mb_only {
            self.interior
                .poly_list_plane_indices
                .push(PlaneIndex::from(0));
            self.interior
                .poly_list_point_indices
                .push(PointIndex::from(0));
            self.interior.poly_list_string_characters.push(0);
            self.interior.hull_plane_indices.push(PlaneIndex::from(0));
            self.interior
                .hull_emit_string_indices
                .push(EmitStringIndex::from(0));
            self.interior.convex_hull_emit_string_characters.push(0);
        } else {
            self.interior.base_ambient_color = ColorI {
                r: self.ambient_color.x as u8,
                g: self.ambient_color.y as u8,
                b: self.ambient_color.z as u8,
                a: 255,
            };
            self.interior.alarm_ambient_color = ColorI {
                r: self.emergency_ambient_color.x as u8,
                g: self.emergency_ambient_color.y as u8,
                b: self.emergency_ambient_color.z as u8,
                a: 255,
            };
            self.process_hull_poly_lists(); // Hull poly lists
            self.compute_lightmaps(); // lightmaps
        }
        // self.calculate_bsp_coverage();
        let balance_factor_save = self.bsp_report.balance_factor;
        self.bsp_report = self.interior.calculate_bsp_raycast_coverage();
        self.bsp_report.balance_factor = balance_factor_save;
        (self.interior, self.bsp_report)
    }

    fn export_brushes(&mut self, progress_report_callback: &mut dyn ProgressEventListener) {
        for i in 0..self.brushes.len() {
            progress_report_callback.progress(
                (i + 1) as u32,
                self.brushes.len() as u32,
                "Exporting convex hulls".to_string(),
                "Exported convex hulls".to_string(),
            );
            self.export_convex_hull(i);
        }
        let (bsp_root, plane_remap) = build_bsp(&self.brushes, progress_report_callback);
        self.bsp_report.balance_factor = bsp_root.balance_factor();
        self.export_bsp_node(&bsp_root, &plane_remap);
        // self.calculate_bsp_raycast_root_coverage(&bsp_root, &plane_remap);
    }

    fn export_bsp_node(&mut self, node: &CSXBSPNode, plane_remap: &Vec<PlaneF>) -> BSPIndex {
        if node.plane_index == None {
            if node.solid {
                let surface_index = self.interior.solid_leaf_surfaces.len() as u32;
                let mut surface_count = 0;
                let mut exported = HashSet::new();
                node.brush_list.iter().for_each(|b| {
                    b.faces.iter().for_each(|f| {
                        let surf_index = self.face_to_surface.get(&f.id).unwrap();
                        if !exported.contains(surf_index.inner()) {
                            surface_count += 1;
                            exported.insert(surf_index.inner());
                            self.interior
                                .solid_leaf_surfaces
                                .push(PossiblyNullSurfaceIndex::NonNull(*surf_index));
                        }
                    });
                });
                if surface_count == 0 {
                    return BSPIndex {
                        leaf: true,
                        solid: false,
                        index: 0,
                    };
                } else {
                    let solid_leaf = BSPSolidLeaf {
                        surface_count: surface_count,
                        surface_index: surface_index.into(),
                    };
                    let leaf_index = self.interior.bsp_solid_leaves.len();
                    self.interior.bsp_solid_leaves.push(solid_leaf);
                    return BSPIndex {
                        leaf: true,
                        solid: true,
                        index: leaf_index as u32,
                    };
                }
            } else {
                let leaf_index = BSPIndex {
                    leaf: true,
                    solid: false,
                    index: 0,
                };
                return leaf_index;
            }
        } else {
            let node_index = self.interior.bsp_nodes.len();
            let bsp_node = BSPNode {
                front_index: BSPIndex {
                    index: 0,
                    leaf: true,
                    solid: false,
                },
                back_index: BSPIndex {
                    index: 0,
                    leaf: true,
                    solid: false,
                },
                plane_index: PlaneIndex::from(0),
            };

            self.interior.bsp_nodes.push(bsp_node);

            let node_plane = &plane_remap[node.plane_index.unwrap() as usize];
            let plane_index = self.export_plane(node_plane);
            let plane_flipped = *plane_index.inner() & 0x8000 != 0;

            let front_index = match node.front {
                Some(ref n) => self.export_bsp_node(n.as_ref(), plane_remap),
                None => BSPIndex {
                    leaf: true,
                    solid: false,
                    index: 0,
                },
            };
            let back_index = match node.back {
                Some(ref n) => self.export_bsp_node(n.as_ref(), plane_remap),
                None => BSPIndex {
                    leaf: true,
                    solid: false,
                    index: 0,
                },
            };
            self.interior.bsp_nodes[node_index].plane_index =
                PlaneIndex::from(*plane_index.inner() & 0x7FFF);
            if plane_flipped {
                self.interior.bsp_nodes[node_index].back_index = front_index;
                self.interior.bsp_nodes[node_index].front_index = back_index;
            } else {
                self.interior.bsp_nodes[node_index].back_index = back_index;
                self.interior.bsp_nodes[node_index].front_index = front_index;
            }

            return BSPIndex {
                leaf: false,
                solid: false,
                index: node_index as u32,
            };
        }
    }

    fn export_point(&mut self, point: &Vertex) -> PointIndex {
        let ord_point = OrdPoint::from(&point.pos);
        if let Some(p) = self.point_map.get(&ord_point) {
            return *p;
        }
        let index = PointIndex::new(self.interior.points.len() as u32);
        self.interior.points.push(point.pos);
        self.interior.point_visibilities.push(0xff);
        self.point_map.insert(ord_point, index);
        return index;
    }

    fn export_tex_gen(&mut self, tex_gen: &TexGen) -> TexGenIndex {
        let index = TexGenIndex::new(self.interior.tex_gen_eqs.len() as _);
        let eq = TexGenEq {
            plane_x: tex_gen.plane_x.clone(),
            plane_y: tex_gen.plane_y.clone(),
        };
        let ord_texgen = OrdTexGen(TexGenEq {
            plane_x: tex_gen.plane_x.clone(),
            plane_y: tex_gen.plane_y.clone(),
        });
        if self.texgen_map.contains_key(&ord_texgen) {
            return *self.texgen_map.get(&ord_texgen).unwrap();
        }
        self.interior.tex_gen_eqs.push(eq);
        self.texgen_map.insert(ord_texgen, index);
        return index;
    }

    fn export_coord_bins(&mut self) {
        // There are always 256 of these (hard-coded in engine)
        for i in 0..256 {
            self.interior.coord_bins.push(CoordBin {
                bin_start: CoordBinIndex::new(i),
                bin_count: 1,
            });
        }
        // Split coordbins into 16x16 equal rect prisms in the xy plane
        // Probably a more efficient way to do this but this will work
        for i in 0..16 {
            let min_x = self.interior.bounding_box.min.x
                + (i as f32 * self.interior.bounding_box.extent().x / 16f32);
            let max_x = self.interior.bounding_box.min.x
                + ((i + 1) as f32 * self.interior.bounding_box.extent().x / 16f32);
            for j in 0..16 {
                let min_y = self.interior.bounding_box.min.y
                    + (j as f32 * self.interior.bounding_box.extent().y / 16f32);
                let max_y = self.interior.bounding_box.min.y
                    + ((j + 1) as f32 * self.interior.bounding_box.extent().y / 16f32);

                let bin_index = (i * 16) + j;
                let mut bin_count = 0;
                self.interior.coord_bins[bin_index as usize].bin_start =
                    CoordBinIndex::new(self.interior.coord_bin_indices.len() as _);
                for (k, hull) in self.interior.convex_hulls.iter().enumerate() {
                    if !(min_x > hull.max_x
                        || max_x < hull.min_x
                        || min_y > hull.max_y
                        || max_y < hull.min_y)
                    {
                        self.interior
                            .coord_bin_indices
                            .push(ConvexHullIndex::new(k as _));
                        bin_count += 1;
                    }
                }

                self.interior.coord_bins[bin_index as usize].bin_count = bin_count as _;
            }
        }
    }

    fn export_texture(&mut self, texture: String) -> TextureIndex {
        for i in 0..self.interior.material_names.len() {
            if self.interior.material_names[i] == texture {
                return TextureIndex::new(i as _);
            }
        }
        let index = TextureIndex::new(self.interior.material_names.len() as _);
        self.interior.material_names.push(texture);
        index
    }

    fn export_plane(&mut self, plane: &PlaneF) -> PlaneIndex {
        assert!(self.interior.planes.len() < 0x10000);
        let pord = OrdPlaneF::from(&plane);

        if self.plane_map.contains_key(&pord) {
            let pval = self.plane_map.get(&pord).unwrap();
            return *pval as PlaneIndex;
        }

        let mut pinvplane = plane.clone();
        pinvplane.normal *= -1.0;
        pinvplane.distance *= -1.0;

        let pord = OrdPlaneF::from(&pinvplane);

        if self.plane_map.contains_key(&pord) {
            let pval = self.plane_map.get(&pord).unwrap();
            let mut pindex = *pval.inner();
            pindex |= 0x8000;
            return PlaneIndex::from(pindex);
        }

        let index = PlaneIndex::new(self.interior.planes.len() as _);

        let normal_ord = OrdPoint::from(&plane.normal);

        let normal_map_idx = self.normal_map.get(&normal_ord);

        match normal_map_idx {
            Some(nidx) => {
                self.interior.planes.push(Plane {
                    normal_index: *nidx,
                    plane_distance: plane.distance,
                });
            }
            None => {
                let normal_index = NormalIndex::new(self.interior.normals.len() as _);
                self.normal_map.insert(normal_ord, normal_index);
                self.interior.normals.push(plane.normal);
                if !self.mb_only {
                    self.interior.normal2s.push(plane.normal);
                }

                self.interior.planes.push(Plane {
                    normal_index: normal_index as _,
                    plane_distance: plane.distance,
                });
            }
        }

        let pord = OrdPlaneF::from(&plane);

        self.plane_map.insert(pord, index);

        index
    }

    fn export_surface(&mut self, face: &Face, hull_points: &Vec<PointIndex>) -> SurfaceIndex {
        if self.face_to_surface.contains_key(&face.face_id) {
            return self.face_to_surface[&face.face_id];
        }
        let index = SurfaceIndex::new(self.interior.surfaces.len() as _);

        self.face_to_surface.insert(face.face_id, index);

        let plane_index = self.export_plane(&face.plane);
        let pflipped = plane_index.inner() & 0x8000 > 0;
        self.face_to_plane.insert(face.face_id, plane_index);

        let tex_gen_index = self.export_tex_gen(&face.texgens);
        let winding_index = WindingIndexIndex::new(self.interior.indices.len() as _);
        let winding_length = face.indices.indices.len();
        for i in 0..winding_length {
            if i >= 2 {
                if i % 2 == 0 {
                    self.interior.indices.push(
                        hull_points
                            [face.indices.indices[winding_length - 1 - (i - 2) / 2] as usize],
                    );
                } else {
                    self.interior
                        .indices
                        .push(hull_points[face.indices.indices[(i + 1) / 2] as usize]);
                }
            } else {
                self.interior
                    .indices
                    .push(hull_points[face.indices.indices[i] as usize]);
            }
        }

        let material_index = self.export_texture(face.material.clone());

        let mut fan_mask = 0b0;
        for i in 0..winding_length {
            fan_mask |= 1 << i;
        }

        let surface = Surface {
            winding_start: winding_index,
            winding_count: winding_length as _,
            plane_index: plane_index,
            plane_flipped: pflipped,
            texture_index: material_index,
            tex_gen_index: tex_gen_index,
            surface_flags: SurfaceFlags::OUTSIDE_VISIBLE,
            fan_mask: fan_mask as _,
            light_map: SurfaceLightMap {
                final_word: 0, // stEnc, lmapLogScaleX, lmapLogScaleY
                tex_gen_x_distance: 0.0,
                tex_gen_y_distance: 0.0,
            },
            light_count: 0,
            light_state_info_start: 0,
            map_offset_x: 0,
            map_offset_y: 0,
            map_size_x: 32,
            map_size_y: 32,
            brush_id: 0,
        };

        //TODO: Figure these out too
        self.interior
            .zone_surfaces
            .push(SurfaceIndex::new(self.interior.surfaces.len() as _));

        self.interior.normal_lmap_indices.push(LMapIndex::new(0u32));
        self.interior
            .alarm_lmap_indices
            .push(LMapIndex::new(0xffffffffu32));
        self.interior.surfaces.push(surface);

        index
    }

    fn export_convex_hull(&mut self, brush_index: usize) -> usize {
        let b = self.brushes[brush_index].clone();
        struct HullPoly {
            pub points: Vec<usize>,
            pub plane_index: usize,
        }
        #[derive(Hash, PartialEq, Eq)]
        struct EmitEdge {
            pub first: usize,
            pub last: usize,
        }

        let index = self.interior.convex_hulls.len();

        let hull_count: usize = b.vertices.vertex.len();
        assert!(hull_count < 0x10000);
        let bounding_box =
            BoxF::from_vertices(&b.vertices.vertex.iter().map(|v| &v.pos).collect::<Vec<_>>());

        let hull = ConvexHull {
            hull_start: HullPointIndex::new(self.interior.hull_indices.len() as _),
            hull_count: hull_count as _,
            min_x: bounding_box.min.x,
            max_x: bounding_box.max.x,
            min_y: bounding_box.min.y,
            max_y: bounding_box.max.y,
            min_z: bounding_box.min.z,
            max_z: bounding_box.max.z,
            surface_start: HullSurfaceIndex::new(self.interior.hull_surface_indices.len() as _),
            surface_count: b.face.len() as _,
            plane_start: HullPlaneIndex::new(self.interior.hull_plane_indices.len() as _),
            poly_list_plane_start: PolyListPlaneIndex::new(
                self.interior.poly_list_plane_indices.len() as _,
            ),
            poly_list_point_start: PolyListPointIndex::new(
                self.interior.poly_list_point_indices.len() as _,
            ),
            poly_list_string_start: PolyListStringIndex::new(0),
            static_mesh: 0,
        };

        let hull_exported_points = b
            .vertices
            .vertex
            .iter()
            .map(|v| self.export_point(v))
            .collect::<Vec<_>>();

        // Export hull points

        self.interior
            .hull_indices
            .append(&mut hull_exported_points.clone());
        if !self.mb_only {
            self.interior
                .poly_list_point_indices
                .append(&mut hull_exported_points.clone());
        }

        // Export hull planes
        let mut hull_plane_indices = b
            .face
            .iter()
            .map(|f| self.export_plane(&f.plane))
            .collect::<Vec<_>>();
        if !self.mb_only {
            self.interior
                .poly_list_plane_indices
                .append(&mut hull_plane_indices.clone());
            self.interior
                .hull_plane_indices
                .append(&mut hull_plane_indices);
        }

        // Export hull surfaces
        let mut hull_surface_indices = b
            .face
            .iter()
            .map(|f| {
                PossiblyNullSurfaceIndex::NonNull(self.export_surface(f, &hull_exported_points))
            })
            .collect::<Vec<_>>();
        self.interior
            .hull_surface_indices
            .append(&mut hull_surface_indices);

        // Hull polys
        let mut hull_polys = vec![];
        b.face.iter().for_each(|face| {
            let mut points = vec![];
            for i in 0..face.indices.indices.len() {
                points.push(face.indices.indices[i] as usize);
            }
            hull_polys.push(HullPoly {
                points: points.into_iter().map(|p| p).collect::<Vec<_>>(),
                plane_index: *self.face_to_plane[&face.face_id].inner() as usize,
            });
        });

        // Ok, now we have to construct an emit string for each vertex.  This should be fairly
        //  straightforward, the procedure is:
        // for each point:
        //   - find all polys that contain that point
        //   - find all points in those polys
        //   - find all edges  in those polys
        //   - enter the string
        //  The tricky bit is that we have to set up the emit indices to be relative to the
        //   hullindices.
        for (i, _) in b.vertices.vertex.into_iter().enumerate() {
            let mut emit_poly_indices = vec![];
            if !self.mb_only {
                // Collect emitted polys for this point
                for (j, poly) in hull_polys.iter().enumerate() {
                    if poly.points.contains(&i) {
                        emit_poly_indices.push(j);
                    }
                }
                // We also have to emit any polys that share the plane, but not necessarily the
                //  support point
                let mut new_indices = vec![];
                for (j, poly) in hull_polys.iter().enumerate() {
                    for &emit_poly in emit_poly_indices.iter() {
                        if emit_poly == j {
                            continue;
                        }

                        if hull_polys[emit_poly].plane_index == poly.plane_index {
                            if emit_poly_indices.contains(&j) {
                                continue;
                            }
                            new_indices.push(j);
                        }
                    }
                }
                emit_poly_indices.extend(new_indices);

                assert_ne!(emit_poly_indices.len(), 0);

                // Then generate all points and edges these polys contain
                let emit_points: Vec<usize> = Vec::from_iter(
                    emit_poly_indices
                        .iter()
                        .flat_map(|&poly| hull_polys[poly].points.clone())
                        .collect::<HashSet<_>>()
                        .into_iter(),
                );
                let emit_edges: Vec<EmitEdge> = Vec::from_iter(
                    emit_poly_indices
                        .iter()
                        .flat_map(|&poly| {
                            windows2_wrap(&hull_polys[poly].points).into_iter().map(
                                |(&first, &second)| EmitEdge {
                                    first: first.min(second),
                                    last: first.max(second),
                                },
                            )
                        })
                        .collect::<HashSet<_>>()
                        .into_iter(),
                );

                let mut emit_string: Vec<u8> = vec![];
                emit_string.push(emit_points.len() as _);
                for &point in &emit_points {
                    assert!(point < 0x100);
                    emit_string.push(point as _);
                }
                emit_string.push(emit_edges.len() as _);
                for edge in emit_edges {
                    assert!(edge.first < 0x100);
                    assert!(edge.last < 0x100);
                    emit_string.push(edge.first as _);
                    emit_string.push(edge.last as _);
                }
                emit_string.push(emit_poly_indices.len() as _);
                for poly_index in emit_poly_indices {
                    assert!(hull_polys[poly_index].points.len() < 0x100);
                    assert!(poly_index < 0x100);
                    emit_string.push(hull_polys[poly_index].points.len() as _);
                    emit_string.push(poly_index as _);
                    for point in hull_polys[poly_index].points.iter() {
                        if let Some(point_index) = emit_points.iter().position(|pt| pt == point) {
                            assert!(point_index < 0x100);
                            emit_string.push(point_index as _);
                        }
                    }
                }

                let emit_string_index = self.export_emit_string(emit_string);
                self.interior
                    .hull_emit_string_indices
                    .push(emit_string_index as _);
            }
        }

        self.interior.convex_hulls.push(hull);
        index
    }

    fn process_hull_poly_lists(&mut self) {
        self.interior.poly_list_plane_indices.clear();
        self.interior.poly_list_point_indices.clear();
        self.interior.poly_list_string_characters.clear();
        for hull in self.interior.convex_hulls.iter_mut() {
            let mut point_indices: Vec<u32> = vec![];
            let mut plane_indices: Vec<u16> = vec![];
            let mut temp_surfaces = vec![];

            // Extract all the surfaces from this hull into our temporary processing format
            for i in 0..hull.surface_count {
                let mut temp_surface = TempProcSurface::new();
                let surface_index = &self.interior.hull_surface_indices
                    [(i as u32 + hull.surface_start.inner()) as usize];
                {
                    match surface_index {
                        PossiblyNullSurfaceIndex::Null(idx) => {
                            let ns = &self.interior.null_surfaces[*idx.inner() as usize];
                            temp_surface.plane_index = *ns.plane_index.inner();
                            temp_surface.num_points = ns.winding_count as usize;
                            for j in 0..ns.winding_count {
                                temp_surface.point_indices[j as usize] = *self.interior.indices
                                    [*ns.winding_start.inner() as usize + j as usize]
                                    .inner();
                            }
                        }
                        PossiblyNullSurfaceIndex::NonNull(idx) => {
                            let s = &self.interior.surfaces[*idx.inner() as usize];
                            temp_surface.plane_index = *s.plane_index.inner();

                            let mut temp_indices = [0; 32];
                            let mut jdx = 1;
                            let mut j = 1;
                            while j < s.winding_count {
                                temp_indices[jdx] = j;
                                jdx += 1;
                                j += 2;
                            }
                            j = (s.winding_count - 1) & (!1);
                            while j > 0 {
                                temp_indices[jdx] = j;
                                j -= 2;
                            }
                            jdx = 0;
                            for j in 0..s.winding_count {
                                if s.fan_mask & (1 << j) > 0 {
                                    temp_surface.point_indices[jdx] =
                                        *self.interior.indices[*s.winding_start.inner() as usize
                                            + temp_indices[j as usize] as usize]
                                            .inner();
                                    jdx += 1;
                                }
                            }
                            temp_surface.num_points = jdx;
                        }
                    }
                }
                temp_surfaces.push(temp_surface);
            }

            // First order of business: extract all unique planes and points from
            //  the list of surfaces...
            for surf in temp_surfaces.iter() {
                let mut found = false;
                for plane_index in plane_indices.iter() {
                    if surf.plane_index == *plane_index {
                        found = true;
                        break;
                    }
                }
                if !found {
                    plane_indices.push(surf.plane_index);
                }
                for k in 0..surf.num_points {
                    found = false;
                    for point_index in point_indices.iter() {
                        if *point_index == surf.point_indices[k] {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        point_indices.push(surf.point_indices[k]);
                    }
                }
            }

            // Now that we have all the unique points and planes, remap the surfaces in
            //  terms of the offsets into the unique point list...
            for surf in temp_surfaces.iter_mut() {
                for k in 0..surf.num_points {
                    let mut found = false;
                    for l in 0..point_indices.len() {
                        if point_indices[l] == surf.point_indices[k] {
                            surf.point_indices[k] = l as u32;
                            found = true;
                            break;
                        }
                    }
                    assert!(
                        found,
                        "Error remapping point indices in interior collision processing"
                    );
                }
            }

            // Ok, at this point, we have a list of unique points, unique planes, and the
            //  surfaces all remapped in those terms.  We need to check our error conditions
            //  that will make sure that we can properly encode this hull:
            assert!(
                plane_indices.len() < 256,
                "Error, > 256 planes on an interior hull"
            );
            assert!(
                point_indices.len() < 65536,
                "Error, > 65536 points on an interior hull"
            );
            assert!(
                temp_surfaces.len() < 256,
                "Error, > 256 surfaces on an interior hull"
            );

            // Now we group the planes together, and merge the closest groups until we're left
            //  with <= 8 groups
            let mut plane_groups = vec![];
            for plane_index in plane_indices.iter() {
                let mut pg = PlaneGrouping::new();
                pg.num_planes = 1;
                pg.plane_indices[0] = *plane_index;
                plane_groups.push(pg);
            }

            while plane_groups.len() > 8 {
                // Find the two closest groups.  If mdp(i, j) is the value of the
                //  largest pairwise dot product that can be computed from the vectors
                //  of group i, and group j, then the closest group pair is the one
                //  with the smallest value of mdp.
                let mut cur_min = 2.0;
                let mut first_group = -1;
                let mut second_group = -1;

                for j in 0..plane_groups.len() {
                    let first = &plane_groups[j];
                    for k in (j + 1)..plane_groups.len() {
                        let second = &plane_groups[k];
                        let mut max = -2.0;
                        for l in 0..first.num_planes {
                            for m in 0..second.num_planes {
                                let mut first_normal = self.interior.normals[*self.interior.planes
                                    [(first.plane_indices[l] & !0x8000) as usize]
                                    .normal_index
                                    .inner()
                                    as usize]
                                    .clone();
                                if first.plane_indices[l] & 0x8000 > 0 {
                                    first_normal *= -1.0;
                                }
                                let mut second_normal = self.interior.normals[*self.interior.planes
                                    [(second.plane_indices[m] & !0x8000) as usize]
                                    .normal_index
                                    .inner()
                                    as usize]
                                    .clone();
                                if second.plane_indices[m] & 0x8000 > 0 {
                                    second_normal *= -1.0;
                                }
                                let normal_dot = first_normal.dot(second_normal);
                                if normal_dot > max {
                                    max = normal_dot;
                                }
                            }
                        }

                        if max < cur_min {
                            cur_min = max;
                            first_group = j as i32;
                            second_group = k as i32;
                        }
                    }
                }
                assert!(
                    first_group != -1 && second_group != -1,
                    "Error, unable to find a suitable pairing?"
                );

                // Merge first and second
                let mut from = plane_groups[second_group as usize].clone();
                let to = &mut plane_groups[first_group as usize];
                while from.num_planes != 0 {
                    to.plane_indices[to.num_planes] = from.plane_indices[from.num_planes - 1];
                    to.num_planes += 1;
                    from.num_planes -= 1;
                }

                // And remove the merged group
                plane_groups.remove(second_group as usize);
            }

            // Assign a mask to each of the plane groupings
            for (j, plane_group) in plane_groups.iter_mut().enumerate() {
                plane_group.mask = (1 << j) as u8;
            }

            // Now, assign the mask to each of the temp polys
            for surf in temp_surfaces.iter_mut() {
                let mut assigned = false;
                for plane_group in plane_groups.iter() {
                    for l in 0..plane_group.num_planes {
                        if plane_group.plane_indices[l] == surf.plane_index {
                            surf.mask = plane_group.mask;
                            assigned = true;
                            break;
                        }
                    }
                    if assigned {
                        break;
                    }
                }
                assert!(
                    assigned,
                    "Error, missed a plane somewhere in the hull poly list!"
                );
            }

            // Copy the appropriate group mask to the plane masks
            let mut plane_masks = vec![];
            for plane_index in plane_indices.iter() {
                let mut found = false;
                for plane_group in plane_groups.iter() {
                    for l in 0..plane_group.num_planes {
                        if plane_group.plane_indices[l] == *plane_index {
                            plane_masks.push(plane_group.mask);
                            found = true;
                            break;
                        }
                    }
                    if found {
                        break;
                    }
                }
                if !found {
                    plane_masks.push(0);
                }
            }

            // And whip through the points, constructing the total mask for that point
            let mut point_masks = vec![];
            for (j, _) in point_indices.iter().enumerate() {
                point_masks.push(0);
                for surf in temp_surfaces.iter() {
                    for l in 0..surf.num_points {
                        if surf.point_indices[l] == j as u32 {
                            point_masks[j] |= surf.mask;
                            break;
                        }
                    }
                }
            }

            // Create the emit strings, and we're done!

            // Set the range of planes
            hull.poly_list_plane_start =
                PolyListPlaneIndex::from(self.interior.poly_list_plane_indices.len() as u32);

            for plane_index in plane_indices.iter() {
                self.interior
                    .poly_list_plane_indices
                    .push(PlaneIndex::from(*plane_index));
            }

            // Set the range of points
            hull.poly_list_point_start =
                PolyListPointIndex::from(self.interior.poly_list_point_indices.len() as u32);
            for point_index in point_indices.iter() {
                self.interior
                    .poly_list_point_indices
                    .push(PointIndex::from(*point_index));
            }

            // Now the emit string.  The emit string goes like: (all fields are bytes)
            //  NumPlanes (PLMask) * NumPlanes
            //  NumPointsHi NumPointsLo (PtMask) * NumPoints
            //  NumSurfaces
            //   (NumPoints SurfaceMask PlOffset (PtOffsetHi PtOffsetLo) * NumPoints) * NumSurfaces
            //
            let mut _string_len = 1 + plane_indices.len() + 2 + point_indices.len() + 1;
            for surf in temp_surfaces.iter() {
                _string_len += 1 + 1 + 1 + (surf.num_points * 2);
            }

            hull.poly_list_string_start =
                PolyListStringIndex::from(self.interior.poly_list_string_characters.len() as u32);

            // Planes
            self.interior
                .poly_list_string_characters
                .push(plane_indices.len() as u8);
            for plane_index in plane_masks.iter() {
                self.interior.poly_list_string_characters.push(*plane_index);
            }

            // Points
            self.interior
                .poly_list_string_characters
                .push(((point_indices.len() >> 8) & 0xFF) as u8);
            self.interior
                .poly_list_string_characters
                .push((point_indices.len() & 0xFF) as u8);
            for point_index in point_masks.iter() {
                self.interior.poly_list_string_characters.push(*point_index);
            }

            // Surfaces
            self.interior
                .poly_list_string_characters
                .push(temp_surfaces.len() as u8);
            for surf in temp_surfaces.iter() {
                self.interior
                    .poly_list_string_characters
                    .push(surf.num_points as u8);
                self.interior
                    .poly_list_string_characters
                    .push(surf.mask as u8);

                let mut found = false;
                for (k, plane_index) in plane_indices.iter().enumerate() {
                    if *plane_index == surf.plane_index {
                        self.interior.poly_list_string_characters.push(k as u8);
                        found = true;
                        break;
                    }
                }
                assert!(found, "Error, missed a plane in the poly list!");
                for k in 0..surf.num_points {
                    self.interior
                        .poly_list_string_characters
                        .push(((surf.point_indices[k] >> 8) & 0xFF) as u8);
                    self.interior
                        .poly_list_string_characters
                        .push((surf.point_indices[k] & 0xFF) as u8);
                }
            }
        }
    }

    fn export_emit_string(&mut self, string: Vec<u8>) -> EmitStringIndex {
        let index =
            EmitStringIndex::new(self.interior.convex_hull_emit_string_characters.len() as _);
        if self.emit_string_map.contains_key(&string) {
            return *self.emit_string_map.get(&string).unwrap();
        }
        self.emit_string_map.insert(string.clone(), index);
        self.interior
            .convex_hull_emit_string_characters
            .extend(string);
        index
    }

    fn _calculate_bsp_coverage(&self) {
        let root = &self.interior.bsp_nodes[0];
        let mut used_surfaces = HashSet::new();
        self._calculate_bsp_coverage_rec(root, &mut used_surfaces);
        println!(
            "BSP Coverage: {} / {} surfaces ({}%)",
            used_surfaces.len(),
            self.interior.surfaces.len(),
            (used_surfaces.len() as f32 / self.interior.surfaces.len() as f32) * 100.0
        );
    }

    fn _calculate_bsp_coverage_rec(&self, bsp_node: &BSPNode, used_surfaces: &mut HashSet<u16>) {
        if bsp_node.front_index.solid && bsp_node.front_index.leaf {
            let leaf = &self.interior.bsp_solid_leaves[bsp_node.front_index.index as usize];
            let surfaces = &self.interior.solid_leaf_surfaces[(*leaf.surface_index.inner() as usize)
                ..((*leaf.surface_index.inner() + leaf.surface_count as u32) as usize)];
            surfaces.iter().for_each(|s| match s {
                PossiblyNullSurfaceIndex::NonNull(s_inner) => {
                    used_surfaces.insert(*s_inner.inner());
                }
                _ => {}
            });
        } else if !bsp_node.front_index.leaf {
            self._calculate_bsp_coverage_rec(
                &self.interior.bsp_nodes[bsp_node.front_index.index as usize],
                used_surfaces,
            );
        }
        if bsp_node.back_index.solid && bsp_node.back_index.leaf {
            let leaf = &self.interior.bsp_solid_leaves[bsp_node.back_index.index as usize];
            let surfaces = &self.interior.solid_leaf_surfaces[(*leaf.surface_index.inner() as usize)
                ..((*leaf.surface_index.inner() + leaf.surface_count as u32) as usize)];
            surfaces.iter().for_each(|s| match s {
                PossiblyNullSurfaceIndex::NonNull(s_inner) => {
                    used_surfaces.insert(*s_inner.inner());
                }
                _ => {}
            });
        } else if !bsp_node.back_index.leaf {
            self._calculate_bsp_coverage_rec(
                &self.interior.bsp_nodes[bsp_node.back_index.index as usize],
                used_surfaces,
            );
        }
    }

    fn _calculate_bsp_raycast_root_coverage(
        &self,
        bsp_root: &CSXBSPNode,
        bsp_plane_list: &[PlaneF],
    ) {
        let mut hit = 0;
        self.interior
            .surfaces
            .iter()
            .enumerate()
            .for_each(|(_, s)| {
                let points = &self.interior.indices[(*s.winding_start.inner() as usize)
                    ..((*s.winding_start.inner() + s.winding_count) as usize)]
                    .iter()
                    .map(|i| self.interior.points[*i.inner() as usize])
                    .collect::<Vec<_>>();
                let mut avg_point: Point3F = points.iter().sum();
                avg_point /= s.winding_count as f32;

                let plane_index = *s.plane_index.inner() & 0x7FFF;
                let norm = self.interior.normals[*self.interior.planes[plane_index as usize]
                    .normal_index
                    .inner() as usize];

                let start = avg_point
                    + (norm
                        * match s.plane_flipped {
                            true => -1.0,
                            false => 1.0,
                        })
                        * 0.1;
                let end = avg_point
                    - (norm
                        * match s.plane_flipped {
                            true => -1.0,
                            false => 1.0,
                        })
                        * 0.1;
                let pidx = usize::MAX;

                if bsp_root.ray_cast(start, end, pidx, bsp_plane_list) {
                    hit += 1;
                } else {
                    // println!("Miss: surface {} plane {}", i, plane_index);
                    // bsp_root.ray_cast(start, end, pidx, bsp_plane_list);
                }
            });
        println!(
            "BSP Raycast Coverage: {} / {} surfaces ({})",
            hit,
            self.interior.surfaces.len(),
            (hit as f32 / self.interior.surfaces.len() as f32) * 100.0
        );
    }

    fn compute_lightmaps(&mut self) {
        let mut rects_to_place: GroupedRectsToPlace<usize, ()> = GroupedRectsToPlace::new();
        let mut lmaps_needed = 1;
        let mut area_remaining = (256 * 256) as i32;

        let mut lmap_surfaces = vec![];

        for surf_idx in 0..self.interior.surfaces.len() {
            let (lmap_area, sc, tc) = self.fill_in_lightmap_info(surf_idx, &mut rects_to_place);
            if area_remaining - lmap_area < 0 {
                lmaps_needed += 1;
                area_remaining = (256 * 256) as i32;
            }

            let mut first_normal = self.interior.normals[*self.interior.planes
                [(self.interior.surfaces[surf_idx].plane_index.into_inner() & !0x8000) as usize]
                .normal_index
                .inner() as usize]
                .clone();
            if (self.interior.surfaces[surf_idx].plane_index.into_inner() & 0x8000) > 0 {
                first_normal *= -1.0;
            }

            // Get these triangle points from the surface for lightmap purposes
            let mut points = vec![];

            let surf = &self.interior.surfaces[surf_idx];
            for k in (surf.winding_start.into_inner() as usize + 2)
                ..(surf.winding_start.into_inner() as usize + surf.winding_count as usize)
            {
                let p1: Point3F;
                let p2: Point3F;
                let p3: Point3F;
                if (k - (surf.winding_start.into_inner() as usize)) % 2 == 0 {
                    p1 = self.interior.points[self.interior.indices[k].into_inner() as usize];
                    p2 = self.interior.points[self.interior.indices[k - 1].into_inner() as usize];
                    p3 = self.interior.points[self.interior.indices[k - 2].into_inner() as usize];
                } else {
                    p1 = self.interior.points[self.interior.indices[k - 2].into_inner() as usize];
                    p2 = self.interior.points[self.interior.indices[k - 1].into_inner() as usize];
                    p3 = self.interior.points[self.interior.indices[k].into_inner() as usize];
                }

                points.push(p1);
                points.push(p2);
                points.push(p3);
            }

            area_remaining -= lmap_area;
            lmap_surfaces.push(LightmapSurface {
                surface_index: surf_idx,
                sc: sc,
                tc: tc,
                dx: 0.0,
                dy: 0.0,
                offset_x: 0,
                offset_y: 0,
                width: 0,
                height: 0,
                normal: first_normal,
                tri_points: points,
                lightmap_index: 0,
            });
        }
        let mut target_bins = BTreeMap::new();
        for i in 0..lmaps_needed {
            target_bins.insert(i, TargetBin::new(256, 256, 255));
        }

        // Pack the lmaps
        let rect_placements = pack_rects(
            &rects_to_place,
            &mut target_bins,
            &volume_heuristic,
            &contains_smallest_box,
        )
        .unwrap();

        // Then pack
        for surf_idx in 0..self.interior.surfaces.len() {
            let (lmap_index, packed_loc) =
                rect_placements.packed_locations().get(&surf_idx).unwrap();
            self.interior.normal_lmap_indices[surf_idx] = LMapIndex::new(*lmap_index);

            self.interior.surfaces[surf_idx].map_size_x = packed_loc.width();
            self.interior.surfaces[surf_idx].map_size_y = packed_loc.height();
            self.interior.surfaces[surf_idx].map_offset_x = packed_loc.x();
            self.interior.surfaces[surf_idx].map_offset_y = packed_loc.y();
            self.interior.surfaces[surf_idx]
                .light_map
                .tex_gen_x_distance += packed_loc.x() as f32 / 256.0;
            self.interior.surfaces[surf_idx]
                .light_map
                .tex_gen_y_distance += packed_loc.y() as f32 / 256.0;
            lmap_surfaces[surf_idx].dx = self.interior.surfaces[surf_idx]
                .light_map
                .tex_gen_x_distance;
            lmap_surfaces[surf_idx].dy = self.interior.surfaces[surf_idx]
                .light_map
                .tex_gen_y_distance;
            lmap_surfaces[surf_idx].width = packed_loc.width() as usize;
            lmap_surfaces[surf_idx].height = packed_loc.height() as usize;
            lmap_surfaces[surf_idx].offset_x = packed_loc.x() as usize;
            lmap_surfaces[surf_idx].offset_y = packed_loc.y() as usize;
            lmap_surfaces[surf_idx].lightmap_index = *lmap_index as usize;
        }

        // Now actually compute the lightmaps
        // Add the lightmaps now
        for _ in 0..lmaps_needed {
            // Add the blank lightmap so we don't crash

            // let lmap_data = lightmap::LightMap::new(
            //     &self.interior,
            //     &lmap_surfaces,
            //     &self.lights,
            //     256,
            //     i as usize,
            //     self.lumel_scale,
            // );

            // self.interior.light_maps.push(LightMap {
            //     light_map: filled_lightmap(&lmap_data.pixels),
            //     light_dir_map: None,
            //     keep_light_map: 0,
            // });

            self.interior.light_maps.push(LightMap {
                light_map: empty_lightmap(
                    self.ambient_color.x as u8,
                    self.ambient_color.y as u8,
                    self.ambient_color.z as u8,
                ),
                light_dir_map: None,
                keep_light_map: 0,
            });
        }
    }

    fn fill_in_lightmap_info(
        &mut self,
        surface_index: usize,
        rects_to_place: &mut GroupedRectsToPlace<usize, ()>,
    ) -> (i32, Point3F, Point3F) {
        let axises = vec![
            Point3F::new(1.0, 0.0, 0.0),
            Point3F::new(0.0, 1.0, 0.0),
            Point3F::new(0.0, 0.0, 1.0),
        ];
        let mut best_dot = -1.0;
        let mut best_index = 0;

        let mut first_normal =
            self.interior.normals[*self.interior.planes[(self.interior.surfaces[surface_index]
                .plane_index
                .into_inner()
                & !0x8000) as usize]
                .normal_index
                .inner() as usize]
                .clone();
        if (self.interior.surfaces[surface_index]
            .plane_index
            .into_inner()
            & 0x8000)
            > 0
        {
            first_normal *= -1.0;
        }

        for i in 0..3 {
            let dot = first_normal.dot(axises[i]).abs();
            if dot > best_dot {
                best_dot = dot;
                best_index = i;
            }
        }
        let sc;
        let tc;

        if axises[best_index].x != 0.0 {
            sc = 1;
            tc = 2;
        } else if axises[best_index].y != 0.0 {
            sc = 0;
            tc = 2;
        } else {
            sc = 0;
            tc = 1;
        }

        let st_enc = match (sc, tc) {
            (0, 1) => 0,
            (0, 2) => 1,
            (1, 0) => 2,
            (1, 2) => 3,
            (2, 0) => 4,
            (2, 1) => 5,
            _ => panic!("Invalid axis combination"),
        };

        let mut coords = vec![];
        let surface = &mut self.interior.surfaces[surface_index];
        for i in 0..surface.winding_count {
            coords.push(
                self.interior.points[self.interior.indices
                    [surface.winding_start.into_inner() as usize + i as usize]
                    .into_inner() as usize]
                    .x,
            );
            coords.push(
                self.interior.points[self.interior.indices
                    [surface.winding_start.into_inner() as usize + i as usize]
                    .into_inner() as usize]
                    .y,
            );
            coords.push(
                self.interior.points[self.interior.indices
                    [surface.winding_start.into_inner() as usize + i as usize]
                    .into_inner() as usize]
                    .z,
            );
        }

        let mut min_s = 1e10;
        let mut min_s_index = 0;

        let mut min_t = 1e10;
        let mut min_t_index = 0;

        let mut max_s = -1e10;
        let mut max_s_index = 0;

        let mut max_t = -1e10;
        let mut max_t_index = 0;

        for i in 0..surface.winding_count {
            if coords[3 * i as usize + sc] < min_s {
                min_s = coords[3 * i as usize + sc];
                min_s_index = i;
            }

            if coords[3 * i as usize + sc] > max_s {
                max_s = coords[3 * i as usize + sc];
                max_s_index = i;
            }

            if coords[3 * i as usize + tc] < min_t {
                min_t = coords[3 * i as usize + tc];
                min_t_index = i;
            }

            if coords[3 * i as usize + tc] > max_t {
                max_t = coords[3 * i as usize + tc];
                max_t_index = i;
            }
        }

        let virtual_min = [
            coords[(min_s_index * 3 + sc as u32) as usize],
            coords[(min_t_index * 3 + tc as u32) as usize],
        ];
        let virtual_max = [
            coords[(max_s_index * 3 + sc as u32) as usize],
            coords[(max_t_index * 3 + tc as u32) as usize],
        ];

        let mut desired_start = [0.0; 2];
        let mut desired_end = [0.0; 2];

        for i in 0..2 {
            desired_start[i] = virtual_min[i] / self.lumel_scale as f32;
            desired_end[i] = virtual_max[i] / self.lumel_scale as f32;
            if desired_start[i] - desired_start[i].floor() < 0.5 {
                desired_start[i] = (desired_start[i] - 1.0).floor();
            } else {
                desired_start[i] = desired_start[i].floor();
            }

            if desired_end[i] - desired_end[i].ceil() < 0.5 {
                desired_end[i] = (desired_end[i] + 1.0).ceil();
            } else {
                desired_end[i] = desired_end[i].ceil();
            }
        }

        let lmap_dim_x = (desired_end[0] - desired_start[0] + 0.5) as u32;
        let lmap_dim_y = (desired_end[1] - desired_start[1] + 0.5) as u32;

        //desired_start[0] *= self.lumel_scale as f32;
        //desired_start[1] *= self.lumel_scale as f32;
        //desired_end[0] *= self.lumel_scale as f32;
        // desired_end[1] *= self.lumel_scale as f32;

        surface.light_map.tex_gen_x_distance = -desired_start[0] / (256.0);
        surface.light_map.tex_gen_y_distance = -desired_start[1] / (256.0);

        let sc_scale = 1.0 / (256.0 * self.lumel_scale as f32);
        let tc_scale = 1.0 / (256.0 * self.lumel_scale as f32);

        let inv_scale_x = ((1.0 / sc_scale) + 0.5) as u32;
        let inv_scale_y = ((1.0 / tc_scale) + 0.5) as u32;
        let log_scale_x = inv_scale_x.ilog2();
        let log_scale_y = inv_scale_y.ilog2();

        self.interior.surfaces[surface_index].light_map.final_word = (st_enc << 13)
            | ((log_scale_x & 0b111111) << 6) as u16
            | (log_scale_y & 0b111111) as u16;
        //  stEnc | logScaleX | logScaleY

        rects_to_place.push_rect(
            surface_index,
            None,
            RectToInsert::new(lmap_dim_x, lmap_dim_y, 255),
        );

        (
            lmap_dim_x as i32 * lmap_dim_y as i32,
            axises[sc] * sc_scale,
            axises[tc] * tc_scale,
        )
    }
}

pub fn windows2_wrap<T>(input: &Vec<T>) -> Vec<(&T, &T)>
where
    T: Copy,
{
    let mut results = vec![];
    for i in 0..input.len() {
        results.push((&input[i], &input[(i + 1) % input.len()]));
    }
    results
}

pub fn get_bounding_box(brushes: &[Brush]) -> BoxF {
    BoxF::from_vertices(
        &brushes
            .iter()
            .flat_map(|t| &t.vertices.vertex)
            .map(|v| &v.pos)
            .collect::<Vec<_>>(),
    )
}

pub fn get_bounding_box_not_owned(brushes: &[&Brush]) -> BoxF {
    BoxF::from_vertices(
        &brushes
            .iter()
            .flat_map(|t| &t.vertices.vertex)
            .map(|v| &v.pos)
            .collect::<Vec<_>>(),
    )
}

fn get_bounding_sphere(brushes: &[Brush]) -> SphereF {
    let b = get_bounding_box(brushes);

    SphereF {
        origin: b.center(),
        radius: (b.max - b.center()).magnitude(),
    }
}

fn empty_interior() -> Interior {
    Interior {
        detail_level: 0,
        min_pixels: 250,
        bounding_box: BoxF {
            min: Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
        bounding_sphere: SphereF {
            origin: Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            radius: 0.0,
        },
        has_alarm_state: 0,
        num_light_state_entries: 0,
        normals: vec![],
        planes: vec![],
        points: vec![],
        point_visibilities: vec![],
        tex_gen_eqs: vec![],
        bsp_nodes: vec![],
        bsp_solid_leaves: vec![],
        material_names: vec![],
        indices: vec![],
        winding_indices: vec![],
        edges: vec![],
        zones: vec![],
        zone_surfaces: vec![],
        zone_static_meshes: vec![],
        zone_portal_lists: vec![],
        portals: vec![],
        surfaces: vec![],
        edge2s: vec![],
        normal2s: vec![],
        normal_indices: vec![],
        normal_lmap_indices: vec![],
        alarm_lmap_indices: vec![],
        null_surfaces: vec![],
        light_maps: vec![],
        solid_leaf_surfaces: vec![],
        animated_lights: vec![],
        light_states: vec![],
        state_datas: vec![],
        state_data_buffers: vec![],
        flags: 0,
        name_buffer_characters: vec![],
        sub_objects: vec![],
        convex_hulls: vec![],
        convex_hull_emit_string_characters: vec![],
        hull_indices: vec![],
        hull_plane_indices: vec![],
        hull_emit_string_indices: vec![],
        hull_surface_indices: vec![],
        poly_list_plane_indices: vec![],
        poly_list_point_indices: vec![],
        poly_list_string_characters: vec![],
        coord_bins: vec![],
        coord_bin_indices: vec![],
        coord_bin_mode: 0,
        base_ambient_color: ColorI {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alarm_ambient_color: ColorI {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        static_meshes: vec![],
        tex_normals: vec![],
        tex_matrices: vec![],
        tex_matrix_indices: vec![],
        extended_light_map_data: 0,
        light_map_border_size: 0,
    }
}

fn empty_lightmap(r: u8, g: u8, b: u8) -> PNG {
    let mut img = ImageBuffer::new(256, 256);
    for (_, _, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgb([r, g, b]);
    }
    let mut v = Vec::new();
    let png = PngEncoder::new(v.by_ref());
    let _ = png
        .write_image(&img, 256, 256, image::ExtendedColorType::Rgb8)
        .unwrap();

    PNG { data: v }
}

fn _filled_lightmap(data: &[u8]) -> PNG {
    let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(256, 256);
    img.copy_from_slice(data);
    let mut v = Vec::new();
    let png = PngEncoder::new(v.by_ref());
    let _ = png
        .write_image(&img, 256, 256, image::ExtendedColorType::Rgb8)
        .unwrap();

    PNG { data: v }
}

struct TempProcSurface {
    pub num_points: usize,
    pub point_indices: [u32; 32],
    pub plane_index: u16,
    pub mask: u8,
}

impl TempProcSurface {
    pub fn new() -> Self {
        TempProcSurface {
            num_points: 0,
            point_indices: [0; 32],
            plane_index: 0,
            mask: 0,
        }
    }
}

#[derive(Copy, Clone)]
struct PlaneGrouping {
    pub num_planes: usize,
    pub plane_indices: [u16; 32],
    pub mask: u8,
}

impl PlaneGrouping {
    pub fn new() -> Self {
        PlaneGrouping {
            num_planes: 0,
            plane_indices: [0; 32],
            mask: 0,
        }
    }
}

#[derive(Clone, PartialOrd)]
pub struct OrdPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl OrdPoint {
    pub fn from(p: &Point3F) -> Self {
        OrdPoint {
            x: p.x,
            y: p.y,
            z: p.z,
        }
    }
}

impl PartialEq for OrdPoint {
    fn eq(&self, other: &Self) -> bool {
        self.x.abs_diff_eq(&other.x, unsafe { POINT_EPSILON })
            && self.y.abs_diff_eq(&other.y, unsafe { POINT_EPSILON })
            && self.z.abs_diff_eq(&other.z, unsafe { POINT_EPSILON })
    }
}

impl Eq for OrdPoint {}

impl Hash for OrdPoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let x = (self.x.floor() as u32 >> 5) & 0xf;
        let y = (self.y.floor() as u32 >> 5) & 0xf;
        let z = (self.z.floor() as u32 >> 5) & 0xf;

        let hash_val = (x << 8) | (y << 4) | z;
        hash_val.hash(state);
    }
}

#[derive(Clone, PartialOrd)]
pub struct OrdPlaneF {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub d: f32,
}

impl OrdPlaneF {
    pub fn from(v: &PlaneF) -> Self {
        OrdPlaneF {
            x: v.normal.x,
            y: v.normal.y,
            z: v.normal.z,
            d: v.distance,
        }
    }
}

impl PartialEq for OrdPlaneF {
    fn eq(&self, other: &Self) -> bool {
        self.x * other.x + self.y * other.y + self.z * other.z > 0.999
            && self.d.abs_diff_eq(&other.d, unsafe { PLANE_EPSILON })
    }
}

impl Eq for OrdPlaneF {}

impl Hash for OrdPlaneF {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut mul = self.x.abs().max(self.y.abs()).max(self.z.abs());
        mul = (mul * 100.0 + 0.5).floor() / 100.0;
        let val = mul * ((self.d.abs() * 100.0 + 0.5).floor() / 100.0);
        let hash_val = (val as u32) % (1 << 12);
        hash_val.hash(state);
    }
}

struct OrdTexGen(TexGenEq);

impl PartialEq for OrdTexGen {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .plane_x
            .normal
            .x
            .abs_diff_eq(&other.0.plane_x.normal.x, 1e-5)
            && self
                .0
                .plane_x
                .normal
                .y
                .abs_diff_eq(&other.0.plane_x.normal.y, 1e-5)
            && self
                .0
                .plane_x
                .normal
                .z
                .abs_diff_eq(&other.0.plane_x.normal.z, 1e-5)
            && self
                .0
                .plane_x
                .distance
                .abs_diff_eq(&other.0.plane_x.distance, 1e-5)
            && self
                .0
                .plane_y
                .normal
                .x
                .abs_diff_eq(&other.0.plane_y.normal.x, 1e-5)
            && self
                .0
                .plane_y
                .normal
                .y
                .abs_diff_eq(&other.0.plane_y.normal.y, 1e-5)
            && self
                .0
                .plane_y
                .normal
                .z
                .abs_diff_eq(&other.0.plane_y.normal.z, 1e-5)
            && self
                .0
                .plane_y
                .distance
                .abs_diff_eq(&other.0.plane_y.distance, 1e-5)
    }
}

impl Eq for OrdTexGen {}

impl Hash for OrdTexGen {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut mul = self
            .0
            .plane_x
            .normal
            .x
            .abs()
            .max(self.0.plane_x.normal.y.abs())
            .max(self.0.plane_x.normal.z.abs());
        mul = (mul * 100.0 + 0.5).floor() / 100.0;
        let val = mul * ((self.0.plane_x.distance.abs() * 100.0 + 0.5).floor() / 100.0);
        let hash_val = (val as u32) % (1 << 12);
        hash_val.hash(state);

        // Same for plane y
        let mut mul = self
            .0
            .plane_y
            .normal
            .x
            .abs()
            .max(self.0.plane_y.normal.y.abs())
            .max(self.0.plane_y.normal.z.abs());

        mul = (mul * 100.0 + 0.5).floor() / 100.0;
        let val = mul * ((self.0.plane_y.distance.abs() * 100.0 + 0.5).floor() / 100.0);
        let hash_val = (val as u32) % (1 << 12);
        hash_val.hash(state);
    }
}

pub trait RaycastCalc {
    fn bsp_ray_cast(
        &self,
        node: &BSPIndex,
        plane_index: &u16,
        start: Point3F,
        end: Point3F,
    ) -> bool;

    fn calculate_bsp_raycast_coverage(&mut self) -> BSPReport;
}

impl RaycastCalc for Interior {
    fn calculate_bsp_raycast_coverage(&mut self) -> BSPReport {
        let mut hit = 0;
        let mut total_surface_area = 0.0;
        let mut hit_surface_area = 0.0;
        self.surfaces.iter().enumerate().for_each(|(_, s)| {
            let points = &self.indices[(*s.winding_start.inner() as usize)
                ..((*s.winding_start.inner() + s.winding_count) as usize)]
                .iter()
                .map(|i| self.points[*i.inner() as usize])
                .collect::<Vec<_>>();
            let mut avg_point: Point3F = points.iter().sum();
            avg_point /= s.winding_count as f32;

            let mut surface_area = 0.0;
            for i in 0..points.len() {
                surface_area += (points[i] - avg_point)
                    .cross(points[(i + 1) % points.len()] - avg_point)
                    .magnitude()
                    / 2.0;
            }
            total_surface_area += surface_area;

            let plane_index = *s.plane_index.inner() & 0x7FFF;
            let norm =
                self.normals[*self.planes[plane_index as usize].normal_index.inner() as usize];

            let start = avg_point
                + (norm
                    * match s.plane_flipped {
                        true => -1.0,
                        false => 1.0,
                    })
                    * 0.1;
            let end = avg_point
                - (norm
                    * match s.plane_flipped {
                        true => -1.0,
                        false => 1.0,
                    })
                    * 0.1;
            let pidx = u16::MAX;
            let start_node_index = BSPIndex {
                index: 0,
                leaf: false,
                solid: false,
            };

            if self.bsp_ray_cast(&start_node_index, &pidx, start, end) {
                hit += 1;
                hit_surface_area += surface_area;
            } else {
                // println!("Miss: surface {} plane {}", i, plane_index);
                // self.bsp_ray_cast(&start_node_index, &pidx, start, end);
            }
        });
        BSPReport {
            hit,
            balance_factor: 0,
            total: self.surfaces.len(),
            hit_area_percentage: (hit_surface_area / total_surface_area) * 100.0,
        }
    }

    fn bsp_ray_cast(
        &self,
        node: &BSPIndex,
        plane_index: &u16,
        start: Point3F,
        end: Point3F,
    ) -> bool {
        if !node.leaf {
            use std::cmp::Ordering;
            let node_value = &self.bsp_nodes[node.index as usize];
            let node_plane_index = *node_value.plane_index.inner();
            let plane_flipped = node_plane_index & 0x8000 > 0;
            let plane_value = &self.planes[(node_plane_index & 0x7FFF) as usize];
            let mut plane_norm = self.normals[*plane_value.normal_index.inner() as usize];
            if plane_flipped {
                plane_norm = -plane_norm;
            }
            let mut plane_d = plane_value.plane_distance;
            if plane_flipped {
                plane_d = -plane_d;
            }

            let s_side_value = plane_norm.dot(start) + plane_d;
            let e_side_value = plane_norm.dot(end) + plane_d;
            let s_side = s_side_value.total_cmp(&0.0);
            let e_side = e_side_value.total_cmp(&0.0);

            match (s_side, e_side) {
                (Ordering::Greater, Ordering::Greater)
                | (Ordering::Greater, Ordering::Equal)
                | (Ordering::Equal, Ordering::Greater) => {
                    self.bsp_ray_cast(&node_value.front_index, &plane_index, start, end)
                }
                (Ordering::Greater, Ordering::Less) => {
                    let intersect_t =
                        (-plane_d - start.dot(plane_norm)) / (end - start).dot(plane_norm);
                    let ip = start + (end - start) * intersect_t;
                    if self.bsp_ray_cast(&node_value.front_index, &plane_index, start, ip) {
                        return true;
                    }
                    self.bsp_ray_cast(
                        &node_value.back_index,
                        node_value.plane_index.inner(),
                        ip,
                        end,
                    )
                }
                (Ordering::Less, Ordering::Greater) => {
                    let intersect_t =
                        (-plane_d - start.dot(plane_norm)) / (end - start).dot(plane_norm);
                    let ip = start + (end - start) * intersect_t;
                    if self.bsp_ray_cast(&node_value.back_index, &plane_index, start, ip) {
                        return true;
                    }
                    self.bsp_ray_cast(
                        &node_value.front_index,
                        node_value.plane_index.inner(),
                        ip,
                        end,
                    )
                }
                (Ordering::Less, Ordering::Less)
                | (Ordering::Less, Ordering::Equal)
                | (Ordering::Equal, Ordering::Less) => {
                    self.bsp_ray_cast(&node_value.back_index, &plane_index, start, end)
                }
                (Ordering::Equal, Ordering::Equal) => {
                    if self.bsp_ray_cast(&node_value.front_index, &plane_index, start, end) {
                        return true;
                    }
                    if self.bsp_ray_cast(&node_value.back_index, &plane_index, start, end) {
                        return true;
                    }
                    false
                }
            }
        } else if node.solid {
            let leaf = &self.bsp_solid_leaves[node.index as usize];
            let surfaces = &self.solid_leaf_surfaces[(*leaf.surface_index.inner() as usize)
                ..((*leaf.surface_index.inner() + leaf.surface_count as u32) as usize)];
            let mut found = 0;
            surfaces.iter().for_each(|s| {
                match s {
                    PossiblyNullSurfaceIndex::NonNull(s_index) => {
                        let surf = &self.surfaces[*s_index.inner() as usize];
                        let surf_plane_index = *surf.plane_index.inner();
                        if surf_plane_index & 0x7FFF == *plane_index & 0x7FFF {
                            found += 1;
                        }
                    }
                    _ => {}
                };
            });
            if found == 0 {
                return false;
            }
            return true;
        } else {
            return false;
        }
    }
}
