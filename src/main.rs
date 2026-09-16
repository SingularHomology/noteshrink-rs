mod arg;
mod kmeans_f;
mod load;
mod processing;
mod save;
mod types;
mod vq;

use colored::*;
use indicatif::ProgressBar;
use load::load_img;
use std::collections::HashMap;

fn main() {
    let options = arg::parse_arguments();
    println!("Starting...");
    let pb = ProgressBar::new(options.filenames.len() as u64);
    pb.inc(0);
    let mut palette_store: HashMap<String, Vec<Vec<u32>>> = HashMap::new();
    for filename in &options.filenames {
        let j = load_img(filename, &options).unwrap();
        pb.inc(1);
        palette_store.insert(filename.clone(), j.2);
    }
    pb.finish_with_message("Done");
    println!("Done!");
    if options.return_palette {
        for (i, j) in palette_store {
            print!("\nPalette for ");
            print!("{}", &i.rsplit('/').next().unwrap());
            print!(": ");
            for k in &j {
                print!("{}", "██".truecolor(k[0] as u8, k[1] as u8, k[2] as u8));
            }
            println!("\n{:?}", j);
        }
    }
}
