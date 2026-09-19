use crate::arg::Options;
use crate::kmeans_f::{apply_kmeans, kmeans_precheck};
use image::{ImageBuffer, RgbImage};
use ndarray::{Array1, Array2, Array3, ArrayD, Axis, IxDyn};
use rand::rng;
use rayon::prelude::*;
use std::collections::HashMap;
use std::ptr::eq;

pub fn sample_pixels(img: &Array3<u8>, option_sample_fraction: usize) -> Array2<u8> {
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

pub fn apply_palette(img: &Array3<u8>, palette: &[Vec<u32>], options: &Options) -> Array2<u8> {
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

pub fn shrink_image(img: &RgbImage, options: &Options) -> (RgbImage, Vec<Vec<u32>>) {
    let (width, height) = img.dimensions();
    let array =
        Array3::from_shape_vec((height as usize, width as usize, 3), img.as_raw().to_vec())
            .unwrap();
    let sample_fraction = options.sample_fraction.parse::<usize>().unwrap_or(5);
    let samples = sample_pixels(&array, sample_fraction);
    let mut palette = get_palette(&samples, options);

    if options.white_bg && !palette.is_empty() {
        palette[0] = vec![255, 255, 255];
    }

    let labels = apply_palette(&array, &palette, options);
    let labels_raw = labels.into_raw_vec_and_offset().0;

    let palette_u8: Vec<[u8; 3]> = palette
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
    (out_img, palette)
}
