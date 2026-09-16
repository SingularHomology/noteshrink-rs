use colored::*;
use indicatif::ProgressBar;
use noteshrink_rs::arg;
use noteshrink_rs::load::load_img;
use rayon::prelude::*;

fn main() {
    let options = arg::parse_arguments();
    println!("Starting...");
    let pb = ProgressBar::new(options.filenames.len() as u64);
    pb.inc(0);
    let results: Vec<(String, Vec<Vec<u32>>)> = options
        .filenames
        .par_iter()
        .map(|filename| {
            let j = load_img(filename, &options).unwrap();
            pb.inc(1);
            (filename.clone(), j.2)
        })
        .collect();
    pb.finish_with_message("Done");
    println!("Done!");
    if options.return_palette {
        for (i, j) in results {
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
