#![allow(unused)]
use cgmath::{Point3, Vector3};

pub struct Bounds3 {
    pub center: Point3<f32>,
    pub half_extent: Vector3<f32>,
}

impl Bounds3 {
    pub const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    const CORNER_SIGNS: [(f32, f32, f32); 8] = [
        (-1.0, -1.0, -1.0),
        (1.0, -1.0, -1.0),
        (1.0, 1.0, -1.0),
        (-1.0, 1.0, -1.0),
        (-1.0, -1.0, 1.0),
        (1.0, -1.0, 1.0),
        (1.0, 1.0, 1.0),
        (-1.0, 1.0, 1.0),
    ];

    pub fn from_size(center: Point3<f32>, size: Vector3<f32>) -> Self {
        Self {
            center,
            half_extent: size * 0.5,
        }
    }

    pub fn from_half_extent(center: Point3<f32>, half_extent: Vector3<f32>) -> Self {
        Self {
            center,
            half_extent,
        }
    }

    pub fn corners(&self) -> [Point3<f32>; 8] {
        Self::CORNER_SIGNS.map(|(x, y, z)| {
            self.center
                + Vector3::new(
                    x * self.half_extent.x,
                    y * self.half_extent.y,
                    z * self.half_extent.z,
                )
        })
    }

    pub fn edge_segments(&self) -> [(Point3<f32>, Point3<f32>); 12] {
        let corners = self.corners();

        Self::EDGES.map(|(start, end)| (corners[start], corners[end]))
    }
}
