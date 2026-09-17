use colored::*;
use indicatif::ProgressBar;
use noteshrink_rs::arg;
use noteshrink_rs::load::load_img;
use noteshrink_rs::pdf::{export_pdf, PdfPage};
use rayon::prelude::*;
use std::path::Path;

fn main() {
    let options = arg::parse_arguments();
    if !options.quiet {
        println!("Starting...");
    }
    let pb = if options.quiet {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(options.filenames.len() as u64)
    };
    pb.inc(0);

    let results: Vec<(String, Vec<Vec<u32>>, PdfPage)> = options
        .filenames
        .par_iter()
        .enumerate()
        .map(|(idx, filename)| {
            let out_png = format!("{}{:04}.png", options.basename, idx);
            let j = load_img(filename, &out_png, &options).unwrap();
            pb.inc(1);
            (filename.clone(), j.2, j.3)
        })
        .collect();

    if !options.quiet {
        pb.finish_with_message("Done");
        println!("Done!");
    }

    if !options.pdfname.is_empty() {
        if !options.quiet {
            println!("Saving PDF to {}...", options.pdfname);
        }
        let pdf_pages: Vec<PdfPage> = results.iter().map(|(_, _, page)| page.clone()).collect();
        export_pdf(&pdf_pages, Path::new(&options.pdfname)).expect("Error exporting PDF");
    }

    if options.return_palette {
        for (i, j, _) in results {
            print!("\nPalette for ");
            print!("{}", i.rsplit('/').next().unwrap());
            print!(": ");
            for k in &j {
                print!("{}", "██".truecolor(k[0] as u8, k[1] as u8, k[2] as u8));
            }
            println!("\n{:?}", j);
        }
    }
}
