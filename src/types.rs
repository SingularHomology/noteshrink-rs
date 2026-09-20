use crate::arg::Options;

pub type RgbColor = [u8; 3];
pub type Palette = Vec<RgbColor>;

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

#[derive(Debug, Clone, PartialEq)]
pub struct ShrinkParams {
    pub num_colors: usize,
    pub value_threshold: f32,
    pub sat_threshold: f32,
    pub sample_fraction: usize,
    pub white_bg: bool,
    pub saturate: bool,
}

impl Default for ShrinkParams {
    fn default() -> Self {
        Self {
            num_colors: 8,
            value_threshold: 25.0,
            sat_threshold: 20.0,
            sample_fraction: 5,
            white_bg: false,
            saturate: true,
        }
    }
}

impl ShrinkParams {
    pub fn whiteboard() -> Self {
        Self {
            value_threshold: 40.0,
            sat_threshold: 15.0,
            white_bg: true,
            ..Default::default()
        }
    }

    pub fn ruled_paper() -> Self {
        Self {
            value_threshold: 15.0,
            sat_threshold: 25.0,
            ..Default::default()
        }
    }

    pub fn high_contrast_bw() -> Self {
        Self {
            num_colors: 2,
            value_threshold: 30.0,
            saturate: false,
            white_bg: true,
            ..Default::default()
        }
    }
}

impl From<&Options> for ShrinkParams {
    fn from(opt: &Options) -> Self {
        Self {
            num_colors: opt.num_colors.parse().unwrap_or(8),
            value_threshold: opt.value_threshold.parse().unwrap_or(25.0),
            sat_threshold: opt.sat_threshold.parse().unwrap_or(20.0),
            sample_fraction: opt.sample_fraction.parse().unwrap_or(5),
            white_bg: opt.white_bg,
            saturate: opt.saturate,
        }
    }
}

impl From<Options> for ShrinkParams {
    fn from(opt: Options) -> Self {
        Self::from(&opt)
    }
}

impl From<&ShrinkParams> for Options {
    fn from(params: &ShrinkParams) -> Self {
        Self {
            num_colors: params.num_colors.to_string(),
            value_threshold: params.value_threshold.to_string(),
            sat_threshold: params.sat_threshold.to_string(),
            sample_fraction: params.sample_fraction.to_string(),
            white_bg: params.white_bg,
            saturate: params.saturate,
            ..Default::default()
        }
    }
}

impl From<ShrinkParams> for Options {
    fn from(params: ShrinkParams) -> Self {
        Self::from(&params)
    }
}
