use crate::arg::Options;
use crate::kmeans_f::{apply_kmeans, kmeans_precheck};
use crate::vq::vq;
use ndarray::{Array1, Array2, Array3, ArrayD, Axis, IxDyn};
use rand::rng;
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

    let points: Vec<Vec<f64>> = samples
        .rows()
        .into_iter()
        .map(|row| row.iter().map(|&val| val as f64).collect())
        .collect();
    let pointsf3: Vec<Vec<f64>> = points
        .iter()
        .enumerate()
        .filter_map(|(i, j)| if fg_mask[i] { Some(j.clone()) } else { None })
        .collect();
    let pointsf2: Vec<f64> = pointsf3.into_iter().flatten().collect();

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
    let bg_color: Vec<u8> = palette.first().unwrap().iter().map(|&x| x as u8).collect();
    let fg_mask = get_fg_mask(
        &Array1::from_shape_vec(3, bg_color).unwrap().into_dyn(),
        &img.clone().into_dyn(),
        options,
    );
    let orig_shape = img.shape();
    let (h, w, c) = img.dim();
    let pixels = img.to_shape((h * w, c)).unwrap();
    let (h, w) = fg_mask
        .clone()
        .into_dimensionality::<ndarray::Ix2>()
        .unwrap()
        .dim();
    let fg_mask2 = fg_mask.to_shape(h * w).unwrap();
    let num_pixels = pixels.shape()[0];
    let centroids: Vec<[f32; 3]> = palette
        .iter()
        .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32])
        .collect();

    let mut pixels_fg: Vec<[f32; 3]> = Vec::with_capacity(num_pixels);
    for (pixel, &is_fg) in pixels.outer_iter().zip(fg_mask2.iter()) {
        if is_fg {
            pixels_fg.push([pixel[0] as f32, pixel[1] as f32, pixel[2] as f32]);
        }
    }

    let closest_centroids = vq(&pixels_fg, &centroids);
    let mut labels: Array1<u8> = Array1::zeros(num_pixels);
    let mut m = 0;
    for (n, &is_fg) in fg_mask2.iter().enumerate() {
        if is_fg {
            labels[n] = closest_centroids[m];
            m += 1;
        }
    }
    let mut o = orig_shape.to_vec();
    o.pop();
    let (x, y) = (o[0], o[1]);
    Array2::from_shape_vec((x, y), labels.into_raw_vec_and_offset().0).unwrap()
}
