use std::{ptr, slice};

use vtracer::{Clustering, Color, Config, FitMode, Hierarchical};

/// C-compatible configuration. Enum values match the C# enums. `simplify = 0`
/// and `max_colors = 0` disable those options; `path_precision = -1` means
/// automatic precision (otherwise 0..=15). Booleans are 0 or 1.
/// Palette entries are packed RGB integers (0xRRGGBB).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VTracerConfig {
    pub clustering: i32,
    pub hierarchical: i32,
    pub mode: i32,
    pub filter_speckle: u32,
    pub color_precision: i32,
    pub layer_difference: i32,
    pub corner_threshold: i32,
    pub splice_threshold: i32,
    pub length_threshold: f64,
    pub max_iterations: u32,
    pub path_precision: i32,
    pub simplify: f64,
    pub max_colors: u32,
    pub optimize: u32,
    pub binary_threshold: u32,
    pub binary_adaptive: u32,
    pub binary_adaptive_window: u32,
    pub watershed_detail: u32,
    pub binary_adaptive_t: f64,
    pub palette: *const u32,
    pub palette_length: usize,
}

impl Default for VTracerConfig {
    fn default() -> Self {
        let config = Config::default();
        Self {
            clustering: 0,
            hierarchical: 0,
            mode: 2,
            filter_speckle: config.filter_speckle as u32,
            color_precision: config.color_precision,
            layer_difference: config.layer_difference,
            corner_threshold: config.corner_threshold,
            splice_threshold: config.splice_threshold,
            length_threshold: config.length_threshold,
            max_iterations: config.max_iterations as u32,
            path_precision: config.path_precision.map_or(-1, |n| n as i32),
            simplify: config.simplify.unwrap_or(0.0),
            max_colors: config.max_colors.unwrap_or(0) as u32,
            optimize: config.optimize as u32,
            binary_threshold: config.binary_threshold as u32,
            binary_adaptive: u32::from(config.binary_adaptive),
            binary_adaptive_window: config.binary_adaptive_window,
            watershed_detail: config.watershed_detail,
            binary_adaptive_t: config.binary_adaptive_t,
            palette: ptr::null(),
            palette_length: 0,
        }
    }
}

impl VTracerConfig {
    pub(crate) unsafe fn to_config(self) -> Result<Config, String> {
        let clustering = match self.clustering {
            0 => Clustering::ColorCluster,
            1 => Clustering::Binary,
            2 => Clustering::Watershed,
            _ => return Err("Invalid clustering".to_owned()),
        };
        let hierarchical = match self.hierarchical {
            0 => Hierarchical::Stacked,
            1 => Hierarchical::Cutout,
            _ => return Err("Invalid hierarchical mode".to_owned()),
        };
        let mode = match self.mode {
            0 => FitMode::Pixel,
            1 => FitMode::Polygon,
            2 => FitMode::Spline,
            _ => return Err("Invalid fit mode".to_owned()),
        };
        for (valid, name) in [
            (self.filter_speckle <= 128, "filter_speckle (0..=128)"),
            (
                (1..=8).contains(&self.color_precision),
                "color_precision (1..=8)",
            ),
            (
                (0..=255).contains(&self.layer_difference),
                "layer_difference (0..=255)",
            ),
            (
                (0..=180).contains(&self.corner_threshold),
                "corner_threshold (0..=180)",
            ),
            (
                (0..=180).contains(&self.splice_threshold),
                "splice_threshold (0..=180)",
            ),
            (
                (3.5..=10.0).contains(&self.length_threshold),
                "length_threshold (3.5..=10)",
            ),
            (self.max_iterations > 0, "max_iterations (positive)"),
            (
                (-1..=15).contains(&self.path_precision),
                "path_precision (-1..=15)",
            ),
            (
                self.simplify.is_finite() && self.simplify >= 0.0,
                "simplify (finite, nonnegative)",
            ),
            (self.optimize <= 2, "optimize (0..=2)"),
            (self.binary_threshold <= 255, "binary_threshold (0..=255)"),
            (self.binary_adaptive <= 1, "binary_adaptive (0 or 1)"),
            (
                (0.0..=100.0).contains(&self.binary_adaptive_t),
                "binary_adaptive_t (0..=100)",
            ),
        ] {
            if !valid {
                return Err(format!("Invalid {name}"));
            }
        }
        if self.palette_length > isize::MAX as usize / size_of::<u32>()
            || (self.palette_length > 0 && self.palette.is_null())
        {
            return Err("Invalid palette buffer".to_owned());
        }
        let palette = if self.palette_length == 0 {
            Vec::new()
        } else {
            unsafe { slice::from_raw_parts(self.palette, self.palette_length) }
                .iter()
                .map(|&rgb| {
                    if rgb > 0xffffff {
                        return Err("Palette entries must be 0xRRGGBB".to_owned());
                    }
                    Ok(Color::new((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(Config {
            clustering,
            hierarchical,
            mode,
            filter_speckle: self.filter_speckle as usize,
            color_precision: self.color_precision,
            layer_difference: self.layer_difference,
            corner_threshold: self.corner_threshold,
            splice_threshold: self.splice_threshold,
            length_threshold: self.length_threshold,
            max_iterations: self.max_iterations as usize,
            path_precision: (self.path_precision >= 0).then_some(self.path_precision as u32),
            simplify: (self.simplify > 0.0).then_some(self.simplify),
            max_colors: (self.max_colors > 0).then_some(self.max_colors as usize),
            optimize: self.optimize as u8,
            binary_threshold: self.binary_threshold as u8,
            binary_adaptive: self.binary_adaptive != 0,
            binary_adaptive_window: self.binary_adaptive_window,
            watershed_detail: self.watershed_detail,
            binary_adaptive_t: self.binary_adaptive_t,
            palette,
        })
    }
}
