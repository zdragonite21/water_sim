use cgmath::{InnerSpace, Point3, Vector3};

pub struct Line3d {
    pub start: Point3<f32>,
    pub end: Point3<f32>,
    pub color: [f32; 4],
    pub width_px: f32,
}

pub struct LineBatch {
    lines: Vec<Line3d>,
}

impl LineBatch {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn push_vector(
        &mut self,
        origin: Point3<f32>,
        vector: Vector3<f32>,
        color: [f32; 4],
        scale: f32,
        width_px: f32,
    ) {
        if vector.magnitude2() <= f32::EPSILON {
            return;
        }

        let end = origin + vector.normalize() * scale;

        self.lines.push(Line3d {
            start: origin,
            end,
            color,
            width_px,
        });
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn reserve_exact(&mut self, size: usize) {
        self.lines.reserve_exact(size);
    }

    pub fn lines(&self) -> &[Line3d] {
        &self.lines
    }

    pub fn push_segment(
        &mut self,
        start: impl Into<Point3<f32>>,
        end: impl Into<Point3<f32>>,
        color: [f32; 4],
        width_px: f32,
    ) {
        let start = start.into();
        let end = end.into();

        if (end - start).magnitude2() <= f32::EPSILON {
            return;
        }

        self.lines.push(Line3d {
            start,
            end,
            color,
            width_px,
        });
    }
}
