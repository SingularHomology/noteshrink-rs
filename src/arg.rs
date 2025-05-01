use clap::Parser;

///Convert scanned, hand-written notes to PDF
#[derive(Debug, Clone, Parser, Default)]
#[command(version, about)]
pub struct Options {
    ///Files to convert
    #[arg(required = true, value_name = "IMAGE", action = clap::ArgAction::Append)]
    pub filenames: Vec<String>,
    ///Reduce program output
    #[arg(short, action = clap::ArgAction::SetTrue, default_value = "true")]
    pub quiet: bool,
    //    ///Output PNG filename base (default: page)
    //    #[arg(short, long, value_name = "BASENAME", default_value = "page")]
    //    pub basename: String,
    //    ///Output PDF filename (default: output.pdf)
    //    #[arg(
    //        short = 'o',
    //        long = "output",
    //        value_name = "PDF",
    //        default_value = "output.pdf"
    //    )]
    //    pub pdfname: String,
    ///Background value threshold percentage (default: 25)
    #[arg(
        short = 'v',
        long = "value-threshold",
        value_name = "PERCENT",
        default_value = "25"
    )]
    pub value_threshold: String,
    ///Background saturation threshold % (default: 20)
    #[arg(
        short = 's',
        long = "saturation-threshold",
        value_name = "PERCENT",
        default_value = "20"
    )]
    pub sat_threshold: String,
    ///Number of output colors (default: 8)
    #[arg(
        short = 'n',
        long = "num-colors",
        value_name = "NUM",
        default_value = "8"
    )]
    pub num_colors: String,
    ///% of pixels to sample (default: 5)
    #[arg(
        short = 'p',
        long = "sample-fraction",
        value_name = "PERCENT",
        default_value = "5"
    )]
    pub sample_fraction: String,
    ///Make background white
    #[arg(short = 'w', action = clap::ArgAction::SetTrue)]
    pub white_bg: bool,
    //    ///Use one global palette for all pages
    //    #[arg(short = 'g', action = clap::ArgAction::SetTrue)]
    //    pub global_palette: bool,
    ///Do not saturate colors
    #[arg(short = 'S', action = clap::ArgAction::SetFalse, default_value = "true")]
    pub saturate: bool,
    //    ///Keep filenames ordered as specified; use if you *really* want IMG_10.png to precede IMG_2.png"
    //    #[arg(short = 'K', action = clap::ArgAction::SetFalse, default_value = "true")]
    //    pub sort_numerically: bool,
    //    ///Set postprocessing command (see -O, -C, -Q)
    //    #[arg(short = 'P', long = "postprocess", value_name = "COMMAND")]
    //    pub postprocess_cmd: Option<String>,
    //    ///Filename suffix/extension for postprocessing command
    //    #[arg(short = 'e', long = "postprocess-ext", default_value = "_post.png")]
    //    pub postprocess_ext: String,
    //    ///Same as -P \"optipng -silent %i -out %o\""
    //    #[arg(short = 'O', action = clap::ArgAction::SetTrue)]
    //    pub optipng: String,
    //    ///Same as -P \"pngcrush -q %i %o\""
    //    #[arg(short = 'C', action = clap::ArgAction::SetTrue)]
    //    pub pngcrush: String,
    //    ///Same as -P \"pngquant --ext %e %i\""
    //    #[arg(short = 'Q', action = clap::ArgAction::SetTrue)]
    //    pub pngquant: String,
    //    ///PDF command (default: \"convert %i %o\")"
    //    #[arg(
    //        short = 'c',
    //        long = "pdf-command",
    //        value_name = "COMMAND",
    //        default_value = "convert %i %o"
    //    )]
    //    pub pdf_cmd: String,
    ///Return the palette that was generated
    #[arg(short = 'r', action = clap::ArgAction::SetTrue, default_value = "false")]
    pub return_palette: bool,
}

pub fn parse_arguments() -> Options {
    Options::parse()
}
