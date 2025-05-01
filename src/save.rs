use crate::arg::Options;
use image::{ImageBuffer, Rgb};
use ndarray::{Array2, ArrayD};

pub fn save(
    output_filename: &str,
    labels: Array2<u8>,
    palette: Vec<Vec<u32>>,
    options: &Options,
) -> Vec<Vec<u32>> {
    let mut palette = palette;
    if !options.quiet {
        println!("  saving ...");
    }
    if options.saturate {
        let rows = palette.len();
        let col = palette[0].len();
        let palettef: Vec<u32> = palette.iter().flat_map(|v| v.iter()).copied().collect();
        let palette2: ArrayD<u32> =
            Array2::from_shape_vec((rows, col), palettef.into_iter().collect())
                .unwrap()
                .into_dyn();
        let pmax = palette2.iter().fold(u32::MIN, |a, &b| a.max(b)) as f32;
        let pmin = palette2.iter().fold(u32::MAX, |a, &b| a.min(b)) as f32;
        let palette3 = 255_f32 * (palette2.mapv(|x| x as f32) - pmin) / (pmax - pmin);
        palette = palette3
            .mapv(|x| x as u32)
            .into_raw_vec_and_offset()
            .0
            .chunks(3)
            .map(|c| c.to_vec())
            .collect();
    }
    if options.white_bg {
        palette[0] = vec![255, 255, 255];
    }
    let binding = labels.clone();
    let orig_shape = binding.shape();
    let labels = labels.into_raw_vec_and_offset().0;
    let mut llabels: Vec<Vec<u8>> = Vec::new();
    for (n, i) in labels.into_iter().enumerate() {
        llabels.insert(
            n,
            palette[i as usize]
                .clone()
                .iter()
                .map(|&x| x as u8)
                .collect(),
        );
    }
    let mut flabels: Vec<u8> = Vec::new();
    for i in llabels {
        for j in i {
            flabels.push(j);
        }
    }
    let img_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(orig_shape[1] as u32, orig_shape[0] as u32, flabels).unwrap();

    img_buffer
        .save(output_filename)
        .expect("Error saving image.");

    match options.return_palette {
        false => Vec::new(),
        true => palette,
    }
}
