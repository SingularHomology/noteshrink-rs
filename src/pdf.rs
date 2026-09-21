use crate::types::{ShrinkParams, DPI};
use miniz_oxide::deflate::compress_to_vec_zlib;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone)]
pub struct PdfPage {
    pub width: u32,
    pub height: u32,
    pub dpi: DPI,
    pub palette: Vec<[u8; 3]>,
    pub labels: Vec<u8>,
}

pub fn export_pdf(
    pages: &[PdfPage],
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if pages.is_empty() {
        return Ok(());
    }

    let compressed_pages: Vec<Vec<u8>> = pages
        .par_iter()
        .map(|page| compress_to_vec_zlib(&page.labels, 6))
        .collect();

    let mut pdf = Pdf::new();
    let catalog_ref = Ref::new(1);
    let pages_root_ref = Ref::new(2);

    let mut next_ref = 3;
    let mut page_meta = Vec::with_capacity(pages.len());
    for _ in pages {
        let p_ref = Ref::new(next_ref);
        let c_ref = Ref::new(next_ref + 1);
        let i_ref = Ref::new(next_ref + 2);
        next_ref += 3;
        page_meta.push((p_ref, c_ref, i_ref));
    }

    pdf.catalog(catalog_ref).pages(pages_root_ref);
    pdf.pages(pages_root_ref)
        .count(pages.len() as i32)
        .kids(page_meta.iter().map(|(p, _, _)| *p));

    for (i, page) in pages.iter().enumerate() {
        let (p_ref, c_ref, i_ref) = page_meta[i];
        let img_name = Name(b"Im0");

        let dpi_x = if page.dpi.x > 0.0 { page.dpi.x } else { 300.0 };
        let dpi_y = if page.dpi.y > 0.0 { page.dpi.y } else { 300.0 };
        let w_pt = (page.width as f32 * 72.0) / dpi_x;
        let h_pt = (page.height as f32 * 72.0) / dpi_y;

        let mut page_obj = pdf.page(p_ref);
        page_obj.parent(pages_root_ref);
        page_obj.media_box(Rect::new(0.0, 0.0, w_pt, h_pt));
        page_obj.contents(c_ref);
        page_obj.resources().x_objects().pair(img_name, i_ref);
        page_obj.finish();

        let mut content = Content::new();
        content.save_state();
        content.transform([w_pt, 0.0, 0.0, h_pt, 0.0, 0.0]);
        content.x_object(img_name);
        content.restore_state();
        pdf.stream(c_ref, &content.finish());

        let flat_palette: Vec<u8> = page.palette.iter().flat_map(|c| *c).collect();
        let mut img_obj = pdf.image_xobject(i_ref, &compressed_pages[i]);
        img_obj.width(page.width as i32);
        img_obj.height(page.height as i32);
        img_obj.bits_per_component(8);
        img_obj.filter(Filter::FlateDecode);

        let color_space = img_obj.color_space();
        color_space.indexed(
            Name(b"DeviceRGB"),
            (page.palette.len() - 1) as i32,
            &flat_palette,
        );

        img_obj.finish();
    }

    let mut file = File::create(output_path)?;
    file.write_all(&pdf.finish())?;
    Ok(())
}

pub fn export_pdf_parallel<F>(
    images: &[image::RgbImage],
    params_list: &[ShrinkParams],
    output_path: &Path,
    on_progress: Option<F>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: FnMut(usize, usize, &str) + Send,
{
    if images.is_empty() {
        return Ok(());
    }

    let total = images.len();
    let progress_mutex = Mutex::new(on_progress);
    let completed = AtomicUsize::new(0);
    let default_params = ShrinkParams::default();

    let pdf_pages: Result<Vec<PdfPage>, Box<dyn std::error::Error + Send + Sync>> = images
        .par_iter()
        .enumerate()
        .map(|(idx, img)| {
            let params = params_list.get(idx).unwrap_or(&default_params);
            let (width, height) = img.dimensions();
            let array = ndarray::ArrayView3::from_shape(
                (height as usize, width as usize, 3),
                img.as_raw(),
            )
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let options = crate::arg::Options::from(params);
            let samples = crate::processing::sample_pixels(array, params.sample_fraction);
            let mut palette = crate::processing::get_palette(&samples, &options);

            if params.white_bg && !palette.is_empty() {
                palette[0] = vec![255, 255, 255];
            }

            let labels = crate::processing::apply_palette(array, &palette, &options);
            let labels_raw = labels.into_raw_vec_and_offset().0;

            let adjusted_palette = crate::save::adjust_palette(palette, &options);
            let palette_u8: Vec<[u8; 3]> = adjusted_palette
                .iter()
                .map(|c| [c[0] as u8, c[1] as u8, c[2] as u8])
                .collect();

            let page = PdfPage {
                width,
                height,
                dpi: DPI::default(),
                palette: palette_u8,
                labels: labels_raw,
            };

            let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
            if let Ok(mut guard) = progress_mutex.lock() {
                if let Some(cb) = guard.as_mut() {
                    let msg = format!("Processed page {} of {}...", done, total);
                    cb(done, total, &msg);
                }
            }

            Ok(page)
        })
        .collect();

    let pages = pdf_pages?;
    export_pdf(&pages, output_path)
}
