use crate::arg::Options;
use ndarray::{Array2, ArrayD};
use png::{BitDepth, ColorType, Encoder};
use std::fs::File;
use std::io::BufWriter;

pub fn adjust_palette(mut palette: Vec<Vec<u32>>, options: &Options) -> Vec<Vec<u32>> {
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
    palette
}

pub fn save(
    output_filename: &str,
    labels: Array2<u8>,
    palette: Vec<Vec<u32>>,
    options: &Options,
) -> Vec<Vec<u32>> {
    let palette = adjust_palette(palette, options);
    if !options.quiet {
        println!("  saving ...");
    }
    let (height, width) = (labels.nrows() as u32, labels.ncols() as u32);
    let labels_raw = labels.into_raw_vec_and_offset().0;

    let flat_palette: Vec<u8> = palette
        .iter()
        .flat_map(|c| [c[0] as u8, c[1] as u8, c[2] as u8])
        .collect();

    let file = File::create(output_filename).expect("Error creating output image file.");
    let mut w = BufWriter::new(file);

    let mut encoder = Encoder::new(&mut w, width, height);
    encoder.set_color(ColorType::Indexed);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_palette(&flat_palette);

    let mut writer = encoder.write_header().expect("Error writing PNG header.");
    writer
        .write_image_data(&labels_raw)
        .expect("Error writing PNG image data.");

    match options.return_palette {
        false => Vec::new(),
        true => palette,
    }
}
