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
    let pb = ProgressBar::new(options.clone().filenames.len().try_into().unwrap());
    pb.inc(0);
    let mut palette_store: HashMap<String, Vec<Vec<u32>>> = HashMap::new();
    for (c, i) in options.clone().filenames.iter().enumerate() {
        let j = load_img(i, &options).unwrap();
        pb.inc(1);
        palette_store.insert(options.clone().filenames[c].clone(), j.2);
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
