#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DPI {
    pub x: f32,
    pub y: f32,
}

impl Default for DPI {
    fn default() -> Self {
        Self { x: 300.0, y: 300.0 }
    }
}

impl DPI {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn uniform(dpi: f32) -> Self {
        Self { x: dpi, y: dpi }
    }
}
