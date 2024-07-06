use notan::math::*;

// honestly mostly copied from Camera2D from macroquad
pub struct Camera {
    /// Rotation in degrees
    pub rotation: f32,
    /// Scaling
    pub zoom: f32,
    /// Rotation and zoom origin
    pub target: Vec2,
    /// Screen space offset
    pub offset: Vec2,

    matrix: Option<Mat3>
}

impl Camera {
    #[allow(dead_code)]
    pub fn from_target(target: Vec2) -> Self {
        Self {
            target,
            ..Default::default()
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.,
            offset: vec2(0.0, 0.0),
            target: vec2(0.0, 0.0),
            rotation: 0.,
            matrix: None,
        }
    }
}

impl Camera {
    pub fn update_matrix(&mut self, window_size: Vec2) {
        let origin = window_size / (2.0 * self.zoom) + -self.offset;
        let mat_origin = Mat3::from_translation(origin);

        let mat_translation = Mat3::from_translation(-self.target);
        let mat_rotation = Mat3::from_rotation_z(self.rotation.to_radians());
        let mat_scale = Mat3::from_scale(vec2(self.zoom, self.zoom));

        self.matrix = Some(mat_scale * mat_origin * mat_rotation * mat_translation);
    }

    pub fn matrix(&self) -> Mat3 {
        self.matrix.expect("attempted to use .matrix() before calling .update_matrix()")
    }

    /// Returns the screen space position for a 2d camera world space position
    /// Screen position in window space - from (0, 0) to (screen_width, screen_height)
    #[allow(dead_code)]
    pub fn world_to_screen(&self, point: Vec2) -> Vec2 {
        let mat = self.matrix();
        mat.transform_point2(point)
    }

    // Returns the world space position for a 2d camera screen space position
    // Point is a screen space position, often mouse x and y
    pub fn screen_to_world(&self, point: Vec2) -> Vec2 {
        let inv_mat = self.matrix().inverse();
        inv_mat.transform_point2(point)
    }
}
