use crate::arg::Options;
use crate::kmeans_f::{apply_kmeans, kmeans_precheck};
use crate::vq::vq;
use ndarray::{s, Array1, Array2, Array3, ArrayD, Axis, IxDyn};
use rand::{rng, seq::SliceRandom};
use std::collections::HashMap;
use std::ptr::eq;

pub fn sample_pixels(img: &Array3<u8>, option_sample_fraction: usize) -> Array2<u8> {
    let (h, w, c) = img.dim();
    let img2 = img.to_shape((h * w, c)).unwrap();
    let num_samples = ((h as f64) * (w as f64) * (option_sample_fraction as f64) * 0.01) as usize;
    let num_pix = h * w;
    let mut idx = ndarray::Array1::range(0., num_pix as f64, 1.);
    (idx.as_slice_mut().unwrap()).shuffle(&mut rng());

    let x = idx.slice(s![..num_samples]);
    let y: Vec<usize> = x.iter().map(|&i| i as usize).collect();
    let z: Vec<_> = y.iter().map(|&i| img2.row(i).to_owned()).collect();
    let z: Vec<_> = z.iter().map(|a| a.view()).collect();
    ndarray::stack(Axis(0), &z).unwrap()
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
    let mut pixels2: Vec<Vec<f32>> = Vec::new();
    for (i, &j) in pixels.outer_iter().zip(fg_mask2.iter()) {
        if j {
            pixels2.push(i.iter().map(|&x| x as f32).collect());
        }
    }
    let mut labels: Array1<u8> = Array1::zeros(num_pixels);
    let pixels_fg: Array2<f32> =
        Array2::from_shape_vec((pixels2.len(), 3), pixels2.concat()).unwrap();
    let centroids_array = Array2::from_shape_vec(
        (palette.len(), 3),
        palette.iter().flat_map(|v| v.iter()).cloned().collect(),
    )
    .unwrap();

    // vq
    let closest_centroids: Vec<u8> = vq(&pixels_fg, &centroids_array);
    let mut m = 0;
    for (n, i) in fg_mask2.iter().enumerate() {
        if *i {
            labels[n] = closest_centroids[m];
            m += 1;
        }
    }
    let mut o = orig_shape.to_vec();
    o.pop();
    let (x, y) = (o[0], o[1]);
    Array2::from_shape_vec((x, y), labels.to_vec()).unwrap()
}
