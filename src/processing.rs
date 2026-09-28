use crate::arg::Options;
use crate::kmeans_f::{apply_kmeans, kmeans_precheck};
use crate::types::{Palette, ShrinkParams};
use image::{imageops::FilterType, ImageBuffer, RgbImage};
use ndarray::{Array1, Array2, ArrayD, ArrayView3, Axis, IxDyn};
use rand::rng;
use rayon::prelude::*;
use std::collections::HashMap;
use std::ptr::eq;

pub fn sample_pixels(img: ArrayView3<'_, u8>, option_sample_fraction: usize) -> Array2<u8> {
    let (h, w, c) = img.dim();
    let num_pix = h * w;
    let num_samples = ((num_pix as f64) * (option_sample_fraction as f64) * 0.01) as usize;

    let sample_indices = rand::seq::index::sample(&mut rng(), num_pix, num_samples);
    let mut sample_data = Vec::with_capacity(num_samples * c);

    for idx in sample_indices.iter() {
        let row = idx / w;
        let col = idx % w;
        sample_data.push(img[[row, col, 0]]);
        sample_data.push(img[[row, col, 1]]);
        sample_data.push(img[[row, col, 2]]);
    }

    Array2::from_shape_vec((num_samples, c), sample_data).unwrap()
}

pub fn rgb_to_sv(rgb: &ArrayD<u8>) -> (ArrayD<f32>, ArrayD<f32>) {
    let axis = rgb.ndim() - 1;
    let cmax = rgb.map_axis(Axis(axis), |x| *x.iter().max().unwrap() as f32);
    let cmin = rgb.map_axis(Axis(axis), |x| *x.iter().min().unwrap() as f32);
    let delta = cmax.clone() - cmin;
    let sat = delta / cmax.clone();
    let saturation = cmax.mapv(|x| if x == 0.0 { 0.0 } else { 1.0 }) * sat;
    let value = cmax / 255.0;

    (saturation.into_dyn(), value.into_dyn())
}

pub fn get_bg_color(img: &Array2<u8>, bits_per_channel: Option<u8>) -> (u8, u8, u8) {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut v: Vec<_> = Vec::new();
    let quantized = quantize(img, bits_per_channel);
    let packed = pack_rgb(&quantized);
    for &value in packed.iter() {
        *counts.entry(value).or_insert(0) += 1;
    }
    for i in counts.keys() {
        v.push(*i);
    }
    let packed_mode = counts.iter().max_by_key(|&(_x, y)| y).unwrap().0;
    let (a, b, c) = unpack_rgb(*packed_mode);
    (a as u8, b as u8, c as u8)
}

pub fn pack_rgb(rgb: &Array2<u8>) -> ArrayD<u32> {
    let mut orig_shape = rgb.shape().to_vec();
    orig_shape.pop();
    if orig_shape.len() == 3 {
        let (h, w, c) = (orig_shape[0], orig_shape[1], orig_shape[2]);
        let rgb = rgb.to_shape((h * w, c)).unwrap();
        let rgbc0 = rgb.column(0);
        let rgbc1 = rgb.column(1);
        let rgbc2 = rgb.column(2);
        let mut result = Vec::new();

        orig_shape.pop();
        for ((&a, &b), &c) in rgbc0.iter().zip(rgbc1.iter()).zip(rgbc2.iter()) {
            let combined = (a as u32) << 16 | (b as u32) << 8 | (c as u32);

            result.push(combined);
        }
        Array2::from_shape_vec((orig_shape[0], orig_shape[1]), result)
            .unwrap()
            .into_dyn()
    } else {
        let rgbc0 = rgb.column(0);
        let rgbc1 = rgb.column(1);
        let rgbc2 = rgb.column(2);
        let mut result = Vec::new();

        for ((&a, &b), &c) in rgbc0.iter().zip(rgbc1.iter()).zip(rgbc2.iter()) {
            let combined = (a as u32) << 16 | (b as u32) << 8 | (c as u32);

            result.push(combined);
        }

        Array1::from_shape_vec(orig_shape[0], result)
            .unwrap()
            .into_dyn()
    }
}

fn unpack_rgb(packed: u32) -> (u32, u32, u32) {
    ((packed >> 16) & 0xff, (packed >> 8) & 0xff, (packed) & 0xff)
}

pub fn quantize(img: &Array2<u8>, bits_per_channel: Option<u8>) -> Array2<u8> {
    let bits_per_channel = match bits_per_channel {
        None => Some(6),
        Some(y) => Some(y),
    };

    let shift = 8 - bits_per_channel.unwrap();
    let halfbin = (1 << shift) >> 1;
    img.mapv(|x| ((x >> shift) << shift) + halfbin)
}

pub fn get_fg_mask(bg_color: &ArrayD<u8>, samples: &ArrayD<u8>, options: &Options) -> ArrayD<bool> {
    let mut shape: Vec<_> = Vec::new();
    for i in samples.shape() {
        if !eq(i, samples.shape().last().unwrap()) {
            shape.push(*i);
        }
    }

    let (s_bg, v_bg) = rgb_to_sv(bg_color);
    let (s_samples, v_samples) = rgb_to_sv(samples);

    let s_diff = (s_bg - s_samples).abs();
    let v_diff = (v_bg - v_samples).abs();

    let p1 = options.sat_threshold.parse::<f32>().unwrap() * 0.01;
    let p2 = options.value_threshold.parse::<f32>().unwrap() * 0.01;
    let t1 = s_diff.mapv(|x| x >= p1);
    let t2 = v_diff.mapv(|x| x >= p2);
    let mut t = Vec::new();
    for (a, b) in t1.iter().zip(t2.iter()) {
        t.push(*a || *b);
    }
    ArrayD::from_shape_vec(IxDyn(&shape), t).unwrap()
}

pub fn get_palette(samples: &Array2<u8>, options: &Options) -> Vec<Vec<u32>> {
    if !options.quiet {
        println!("getting palette...");
    }

    let bg_color = get_bg_color(samples, Some(6));
    let b: ArrayD<u8> = Array1::from_shape_vec(3, vec![bg_color.0, bg_color.1, bg_color.2])
        .unwrap()
        .into_dyn();
    let fg_mask = get_fg_mask(&b, &samples.clone().into_dyn(), options);

    let mut pointsf2 = Vec::with_capacity(samples.nrows() * 3);
    for (i, row) in samples.rows().into_iter().enumerate() {
        if fg_mask[i] {
            pointsf2.push(row[0] as f64);
            pointsf2.push(row[1] as f64);
            pointsf2.push(row[2] as f64);
        }
    }

    apply_kmeans(
        &pointsf2,
        &bg_color,
        options,
        kmeans_precheck(&pointsf2, options),
    )
}

pub fn apply_palette(
    img: ArrayView3<'_, u8>,
    palette: &[Vec<u32>],
    options: &Options,
) -> Array2<u8> {
    if !options.quiet {
        println!("applying palette....");
    }

    let (h, w, _) = img.dim();
    let bg_color = &palette[0];
    let bg_r = bg_color[0] as f32;
    let bg_g = bg_color[1] as f32;
    let bg_b = bg_color[2] as f32;

    let bg_cmax = bg_r.max(bg_g).max(bg_b);
    let bg_cmin = bg_r.min(bg_g).min(bg_b);
    let bg_delta = bg_cmax - bg_cmin;
    let s_bg = if bg_cmax == 0.0 { 0.0 } else { bg_delta / bg_cmax };
    let v_bg = bg_cmax / 255.0;

    let p1 = options.sat_threshold.parse::<f32>().unwrap_or(20.0) * 0.01;
    let p2 = options.value_threshold.parse::<f32>().unwrap_or(25.0) * 0.01;

    let centroids: Vec<[i32; 3]> = palette
        .iter()
        .map(|v| [v[0] as i32, v[1] as i32, v[2] as i32])
        .collect();

    let img_slice = img.as_slice().unwrap();

    let labels: Vec<u8> = img_slice
        .par_chunks_exact(3)
        .map(|rgb| {
            let r_f = rgb[0] as f32;
            let g_f = rgb[1] as f32;
            let b_f = rgb[2] as f32;

            let cmax = r_f.max(g_f).max(b_f);
            let cmin = r_f.min(g_f).min(b_f);
            let delta = cmax - cmin;
            let sat = if cmax == 0.0 { 0.0 } else { delta / cmax };
            let val = cmax / 255.0;

            let is_fg = (s_bg - sat).abs() >= p1 || (v_bg - val).abs() >= p2;

            if !is_fg {
                return 0u8;
            }

            let r = rgb[0] as i32;
            let g = rgb[1] as i32;
            let b = rgb[2] as i32;

            let mut min_d = i32::MAX;
            let mut closest = 0u8;

            for (idx, c) in centroids.iter().enumerate() {
                let dr = r - c[0];
                let dg = g - c[1];
                let db = b - c[2];
                let d = dr * dr + dg * dg + db * db;
                if d < min_d {
                    min_d = d;
                    closest = idx as u8;
                }
            }
            closest
        })
        .collect();

    Array2::from_shape_vec((h, w), labels).unwrap()
}

fn max_filter_1d(src: &[u8], dst: &mut [u8], len: usize, radius: usize) {
    for i in 0..len {
        let start = i.saturating_sub(radius);
        let end = (i + radius + 1).min(len);
        let mut m = src[start];
        for &val in &src[start + 1..end] {
            if val > m {
                m = val;
            }
        }
        dst[i] = m;
    }
}

fn min_filter_1d(src: &[u8], dst: &mut [u8], len: usize, radius: usize) {
    for i in 0..len {
        let start = i.saturating_sub(radius);
        let end = (i + radius + 1).min(len);
        let mut m = src[start];
        for &val in &src[start + 1..end] {
            if val < m {
                m = val;
            }
        }
        dst[i] = m;
    }
}

fn morph_close_channel(channel: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
    let mut dilated_h = vec![0u8; w * h];
    for y in 0..h {
        let offset = y * w;
        max_filter_1d(&channel[offset..offset + w], &mut dilated_h[offset..offset + w], w, radius);
    }

    let mut dilated = vec![0u8; w * h];
    let mut col_buf = vec![0u8; h];
    let mut col_out = vec![0u8; h];
    for x in 0..w {
        for y in 0..h {
            col_buf[y] = dilated_h[y * w + x];
        }
        max_filter_1d(&col_buf, &mut col_out, h, radius);
        for y in 0..h {
            dilated[y * w + x] = col_out[y];
        }
    }

    let mut closed_h = vec![0u8; w * h];
    for y in 0..h {
        let offset = y * w;
        min_filter_1d(&dilated[offset..offset + w], &mut closed_h[offset..offset + w], w, radius);
    }

    let mut closed = vec![0u8; w * h];
    for x in 0..w {
        for y in 0..h {
            col_buf[y] = closed_h[y * w + x];
        }
        min_filter_1d(&col_buf, &mut col_out, h, radius);
        for y in 0..h {
            closed[y * w + x] = col_out[y];
        }
    }

    closed
}

#[derive(Clone, Copy)]
struct XCoord {
    x0: usize,
    x1: usize,
    fx: f32,
}

// Estimates the 2D background illumination surface B(x,y) by running separable
// morphological closing on a 16x downscaled thumbnail (radius 8, effective 272px window).
// Then reconstructs full-resolution background channels via bilinear interpolation and
// inverts lighting via reflectance division: R(x,y) = min(255, (I(x,y) / B(x,y)) * 255).
pub fn normalize_background(img: &RgbImage) -> RgbImage {
    let (orig_w, orig_h) = img.dimensions();
    if orig_w == 0 || orig_h == 0 {
        return img.clone();
    }

    let scale = 16u32;
    let sw = ((orig_w + scale - 1) / scale).max(1) as usize;
    let sh = ((orig_h + scale - 1) / scale).max(1) as usize;

    let raw = img.as_raw();
    let mut ch_r = vec![0u8; sw * sh];
    let mut ch_g = vec![0u8; sw * sh];
    let mut ch_b = vec![0u8; sw * sh];

    for sy in 0..sh {
        let y = ((sy as u32 * scale).min(orig_h - 1)) as usize;
        for sx in 0..sw {
            let x = ((sx as u32 * scale).min(orig_w - 1)) as usize;
            let idx = (y * orig_w as usize + x) * 3;
            let s_idx = sy * sw + sx;
            ch_r[s_idx] = raw[idx];
            ch_g[s_idx] = raw[idx + 1];
            ch_b[s_idx] = raw[idx + 2];
        }
    }

    let radius = 8usize;
    let bg_r = morph_close_channel(&ch_r, sw, sh, radius);
    let bg_g = morph_close_channel(&ch_g, sw, sh, radius);
    let bg_b = morph_close_channel(&ch_b, sw, sh, radius);

    let mut x_coords = Vec::with_capacity(orig_w as usize);
    for x in 0..orig_w {
        let sx = (x as f32 / scale as f32).min((sw - 1) as f32);
        let x0 = sx.floor() as usize;
        let x1 = (x0 + 1).min(sw - 1);
        let fx = sx - x0 as f32;
        x_coords.push(XCoord { x0, x1, fx });
    }

    let mut out_raw = vec![0u8; raw.len()];
    let row_len = orig_w as usize * 3;

    out_raw
        .par_chunks_exact_mut(row_len)
        .enumerate()
        .for_each(|(y, out_row)| {
            let in_row = &raw[y * row_len..(y + 1) * row_len];
            let sy = (y as f32 / scale as f32).min((sh - 1) as f32);
            let y0 = sy.floor() as usize;
            let y1 = (y0 + 1).min(sh - 1);
            let fy = sy - y0 as f32;
            let row0 = y0 * sw;
            let row1 = y1 * sw;

            for (x, xc) in x_coords.iter().enumerate() {
                let px = &in_row[x * 3..x * 3 + 3];
                let fx = xc.fx;

                let idx00 = row0 + xc.x0;
                let idx10 = row0 + xc.x1;
                let idx01 = row1 + xc.x0;
                let idx11 = row1 + xc.x1;

                let r00 = bg_r[idx00] as f32;
                let r10 = bg_r[idx10] as f32;
                let r01 = bg_r[idx01] as f32;
                let r11 = bg_r[idx11] as f32;
                let top_r = r00 + fx * (r10 - r00);
                let bot_r = r01 + fx * (r11 - r01);
                let br = (top_r + fy * (bot_r - top_r)).max(1.0);

                let g00 = bg_g[idx00] as f32;
                let g10 = bg_g[idx10] as f32;
                let g01 = bg_g[idx01] as f32;
                let g11 = bg_g[idx11] as f32;
                let top_g = g00 + fx * (g10 - g00);
                let bot_g = g01 + fx * (g11 - g01);
                let bg = (top_g + fy * (bot_g - top_g)).max(1.0);

                let b00 = bg_b[idx00] as f32;
                let b10 = bg_b[idx10] as f32;
                let b01 = bg_b[idx01] as f32;
                let b11 = bg_b[idx11] as f32;
                let top_b = b00 + fx * (b10 - b00);
                let bot_b = b01 + fx * (b11 - b01);
                let bb = (top_b + fy * (bot_b - top_b)).max(1.0);

                // Reflectance division: R = min(255, (I / B) * 255) per RGB channel
                out_row[x * 3] = ((px[0] as f32 / br) * 255.0).min(255.0) as u8;
                out_row[x * 3 + 1] = ((px[1] as f32 / bg) * 255.0).min(255.0) as u8;
                out_row[x * 3 + 2] = ((px[2] as f32 / bb) * 255.0).min(255.0) as u8;
            }
        });

    ImageBuffer::from_raw(orig_w, orig_h, out_raw).unwrap()
}

pub fn shrink_image_in_memory(img: &RgbImage, params: &ShrinkParams) -> (RgbImage, Palette) {
    let normalized;
    let img = if params.normalize_bg {
        normalized = normalize_background(img);
        &normalized
    } else {
        img
    };
    let (width, height) = img.dimensions();
    let array =
        ArrayView3::from_shape((height as usize, width as usize, 3), img.as_raw()).unwrap();
    let options = Options::from(params);
    let samples = sample_pixels(array, params.sample_fraction);
    let mut palette = get_palette(&samples, &options);

    if params.white_bg && !palette.is_empty() {
        palette[0] = vec![255, 255, 255];
    }

    let labels = apply_palette(array, &palette, &options);
    let labels_raw = labels.into_raw_vec_and_offset().0;

    let palette_u8: Palette = palette
        .iter()
        .map(|c| [c[0] as u8, c[1] as u8, c[2] as u8])
        .collect();

    let out_pixels: Vec<u8> = labels_raw
        .par_iter()
        .flat_map_iter(|&idx| {
            palette_u8
                .get(idx as usize)
                .copied()
                .unwrap_or([255, 255, 255])
        })
        .collect();

    let out_img = ImageBuffer::from_raw(width, height, out_pixels).unwrap();
    (out_img, palette_u8)
}

pub fn shrink_preview(
    img: &RgbImage,
    params: &ShrinkParams,
    max_dimension: u32,
) -> (RgbImage, Palette) {
    let (width, height) = img.dimensions();
    if max_dimension == 0 || (width <= max_dimension && height <= max_dimension) {
        return shrink_image_in_memory(img, params);
    }

    let (new_w, new_h) = if width >= height {
        let h = ((height as u64 * max_dimension as u64) / width as u64).max(1) as u32;
        (max_dimension, h)
    } else {
        let w = ((width as u64 * max_dimension as u64) / height as u64).max(1) as u32;
        (w, max_dimension)
    };

    let resized = image::imageops::resize(img, new_w, new_h, FilterType::Triangle);
    shrink_image_in_memory(&resized, params)
}

pub fn shrink_image(img: &RgbImage, options: &Options) -> (RgbImage, Vec<Vec<u32>>) {
    let params = ShrinkParams::from(options);
    let (out_img, palette_u8) = shrink_image_in_memory(img, &params);
    let palette_u32 = palette_u8
        .into_iter()
        .map(|c| vec![c[0] as u32, c[1] as u32, c[2] as u32])
        .collect();
    (out_img, palette_u32)
}
