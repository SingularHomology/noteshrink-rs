use crate::arg::Options;
use crate::types::ShrinkParams;
use ndarray::{Array2, ArrayD, ArrayView3};
use png::{BitDepth, ColorType, Encoder};
use rayon::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

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

pub fn export_images_parallel<F>(
    images: &[image::RgbImage],
    params_list: &[ShrinkParams],
    base_path: &Path,
    on_progress: Option<F>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnMut(usize, usize, &str) + Send,
{
    if images.is_empty() {
        return Ok(Vec::new());
    }

    let total = images.len();
    let progress_mutex = Mutex::new(on_progress);
    let completed = AtomicUsize::new(0);

    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page");
    let ext = base_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    let parent = base_path.parent().unwrap_or_else(|| Path::new(""));

    let default_params = ShrinkParams::default();

    let paths: Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> = images
        .par_iter()
        .enumerate()
        .map(|(idx, img)| {
            let out_path = if total == 1 {
                parent.join(format!("{}.{}", stem, ext))
            } else {
                parent.join(format!("{}_{:04}.{}", stem, idx, ext))
            };

            let params = params_list.get(idx).unwrap_or(&default_params);
            let (width, height) = img.dimensions();
            let array = ArrayView3::from_shape(
                (height as usize, width as usize, 3),
                img.as_raw(),
            )
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let options = Options::from(params);
            let samples = crate::processing::sample_pixels(array, params.sample_fraction);
            let palette = crate::processing::get_palette(&samples, &options);
            let labels = crate::processing::apply_palette(array, &palette, &options);

            let out_path_str = out_path.to_str().unwrap_or("output.png");
            let _ = save(out_path_str, labels, palette, &options);

            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
            if let Ok(mut guard) = progress_mutex.lock() {
                if let Some(cb) = guard.as_mut() {
                    let msg = format!("Exported image {} of {}...", done, total);
                    cb(done, total, &msg);
                }
            }

            Ok(out_path)
        })
        .collect();

    paths
}
