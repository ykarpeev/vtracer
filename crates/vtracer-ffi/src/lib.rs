use std::ffi::{CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::{ptr, slice};

use vtracer::{ColorImage, Config};

mod config;
pub use config::VTracerConfig;

/// Converts tightly packed, row-major RGBA8 pixels using the default configuration.
///
/// Returns 0 on success, 1 for invalid input/conversion errors, or 2 for a caught
/// panic. Unless `result` is null, it receives a UTF-8 SVG or error message.
/// Release that string with [`vtracer_free_string`].
///
/// # Safety
/// `result` must be null or writable and aligned for one pointer. For valid
/// dimensions and length, `pixels` must point to `length` readable bytes,
/// unchanged for the duration of this call, and must not overlap `result`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vtracer_trace_rgba(
    pixels: *const u8,
    length: usize,
    width: u32,
    height: u32,
    result: *mut *mut c_char,
) -> i32 {
    unsafe { vtracer_trace_rgba_with_config(pixels, length, width, height, ptr::null(), result) }
}

/// Converts RGBA8 pixels with custom settings. A null `config` uses defaults.
/// Return codes and string ownership match [`vtracer_trace_rgba`].
///
/// # Safety
/// All safety requirements of [`vtracer_trace_rgba`] apply. `config` must be null
/// or point to a readable, aligned [`VTracerConfig`]. Its palette must be null
/// with zero length, or point to that many readable, aligned RGB entries.
/// Configuration and palette memory must remain unchanged during this call
/// and must not overlap `result`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vtracer_trace_rgba_with_config(
    pixels: *const u8,
    length: usize,
    width: u32,
    height: u32,
    config: *const VTracerConfig,
    result: *mut *mut c_char,
) -> i32 {
    if result.is_null() {
        return 1;
    }
    unsafe { result.write(ptr::null_mut()) };

    let conversion = catch_unwind(AssertUnwindSafe(|| {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .filter(|&n| n <= isize::MAX as usize);
        if width == 0
            || height == 0
            || width > i32::MAX as u32
            || height > i32::MAX as u32
            || expected != Some(length)
            || pixels.is_null()
        {
            return Err(
                "Expected width * height * 4 RGBA bytes and positive dimensions".to_owned(),
            );
        }

        let config = if config.is_null() {
            Config::default()
        } else {
            unsafe { (*config).to_config() }?
        };
        let image = ColorImage {
            pixels: unsafe { slice::from_raw_parts(pixels, length) }.to_vec(),
            width: width as usize,
            height: height as usize,
        };
        config
            .build()
            .and_then(|pipeline| pipeline.to_svg(&image))
            .map_err(|error| error.to_string())
    }));

    let (status, text) = match conversion {
        Ok(Ok(svg)) => (0, svg),
        Ok(Err(message)) => (1, message),
        Err(_) => (2, "vtracer panicked during conversion".to_owned()),
    };
    match CString::new(text) {
        Ok(text) => {
            unsafe { result.write(text.into_raw()) };
            status
        }
        Err(_) => {
            unsafe {
                result.write(
                    c"vtracer returned a string containing a null byte"
                        .to_owned()
                        .into_raw(),
                )
            };
            1
        }
    }
}

/// Releases an SVG or error message returned by [`vtracer_trace_rgba`].
///
/// # Safety
/// `value` must be null or an unmodified pointer returned by this library that
/// has not already been freed. It must not be accessed after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vtracer_free_string(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn converts_rgba_to_svg() {
        let pixels = [255, 0, 0, 255].repeat(32 * 32);
        let mut result = ptr::null_mut();
        let status =
            unsafe { vtracer_trace_rgba(pixels.as_ptr(), pixels.len(), 32, 32, &mut result) };
        assert_eq!(status, 0);
        let svg = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));
        unsafe { vtracer_free_string(result) };
    }

    #[test]
    fn rejects_invalid_buffers_without_reading_them() {
        for (pixels, length, width, height) in [
            (ptr::null(), 4, 1, 1),
            (ptr::null(), 0, 0, 1),
            (ptr::null(), 3, 1, 1),
            (ptr::null(), usize::MAX, u32::MAX, u32::MAX),
        ] {
            let mut result = ptr::null_mut();
            let status = unsafe { vtracer_trace_rgba(pixels, length, width, height, &mut result) };
            assert_eq!(status, 1);
            assert!(!result.is_null());
            let error = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
            assert!(error.contains("RGBA"));
            unsafe { vtracer_free_string(result) };
        }
    }

    #[test]
    fn accepts_null_output_and_null_free_without_dereferencing() {
        assert_eq!(
            unsafe { vtracer_trace_rgba(ptr::null(), 0, 0, 0, ptr::null_mut()) },
            1
        );
        unsafe { vtracer_free_string(ptr::null_mut()) };
    }

    fn trace_with_config(config: &VTracerConfig) -> (i32, String) {
        let pixels = [255, 0, 0, 255].repeat(32 * 32);
        let mut result = ptr::null_mut();
        let status = unsafe {
            vtracer_trace_rgba_with_config(
                pixels.as_ptr(),
                pixels.len(),
                32,
                32,
                config,
                &mut result,
            )
        };
        let text = unsafe { CStr::from_ptr(result) }
            .to_str()
            .unwrap()
            .to_owned();
        unsafe { vtracer_free_string(result) };
        (status, text)
    }

    #[test]
    fn configured_conversion_matches_rust_pipeline() {
        let palette = [0x00ff00];
        let native = VTracerConfig {
            hierarchical: 1,
            mode: 0,
            filter_speckle: 0,
            color_precision: 8,
            layer_difference: 32,
            corner_threshold: 90,
            splice_threshold: 60,
            length_threshold: 5.0,
            max_iterations: 8,
            path_precision: -1,
            simplify: 1.5,
            max_colors: 2,
            optimize: 0,
            palette: palette.as_ptr(),
            palette_length: palette.len(),
            ..VTracerConfig::default()
        };
        let expected = Config {
            hierarchical: vtracer::Hierarchical::Cutout,
            mode: vtracer::FitMode::Pixel,
            filter_speckle: 0,
            color_precision: 8,
            layer_difference: 32,
            corner_threshold: 90,
            splice_threshold: 60,
            length_threshold: 5.0,
            max_iterations: 8,
            path_precision: None,
            simplify: Some(1.5),
            max_colors: Some(2),
            optimize: 0,
            palette: vec![vtracer::Color::new(0, 255, 0)],
            ..Config::default()
        };
        let image = ColorImage {
            pixels: [255, 0, 0, 255].repeat(32 * 32),
            width: 32,
            height: 32,
        };
        let (status, svg) = trace_with_config(&native);
        assert_eq!(status, 0, "{svg}");
        assert_eq!(svg, expected.build().unwrap().to_svg(&image).unwrap());
        assert!(svg.to_lowercase().contains("#00ff00"));

        let (status, default_svg) = trace_with_config(&VTracerConfig::default());
        assert_eq!(status, 0);
        assert_eq!(
            default_svg,
            Config::default().build().unwrap().to_svg(&image).unwrap()
        );
        assert_ne!(svg, default_svg);
    }

    #[test]
    fn binary_threshold_changes_the_result() {
        let mut config = VTracerConfig {
            clustering: 1,
            mode: 0,
            filter_speckle: 0,
            binary_threshold: 0,
            ..VTracerConfig::default()
        };
        let (status, empty_svg) = trace_with_config(&config);
        assert_eq!(status, 0);
        assert!(!empty_svg.contains("<path"));
        config.binary_threshold = 255;
        let (status, foreground_svg) = trace_with_config(&config);
        assert_eq!(status, 0);
        assert!(foreground_svg.contains("<path"));
    }

    #[test]
    fn rejects_invalid_configuration_with_an_owned_error() {
        let invalid_palette = [0xff000000];
        for config in [
            VTracerConfig {
                clustering: 3,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                hierarchical: -1,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                mode: 3,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                color_precision: 0,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                simplify: f64::NAN,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                length_threshold: f64::INFINITY,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                binary_adaptive: 2,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                binary_adaptive_t: f64::NAN,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                path_precision: 16,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                palette_length: 1,
                ..VTracerConfig::default()
            },
            VTracerConfig {
                palette: invalid_palette.as_ptr(),
                palette_length: invalid_palette.len(),
                ..VTracerConfig::default()
            },
        ] {
            let (status, error) = trace_with_config(&config);
            assert_eq!(status, 1, "{error}");
            assert!(!error.is_empty());
        }
    }
}
