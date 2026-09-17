use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(version, about)]
pub struct Options {
    #[arg(required = true, value_name = "IMAGE", action = clap::ArgAction::Append)]
    pub filenames: Vec<String>,
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub quiet: bool,
    #[arg(short, long, value_name = "BASENAME", default_value = "page")]
    pub basename: String,
    #[arg(
        short = 'o',
        long = "output",
        value_name = "PDF",
        default_value = "output.pdf"
    )]
    pub pdfname: String,
    #[arg(long = "dpi", value_name = "DPI")]
    pub dpi: Option<f32>,
    #[arg(
        short = 'v',
        long = "value-threshold",
        value_name = "PERCENT",
        default_value = "25"
    )]
    pub value_threshold: String,
    #[arg(
        short = 's',
        long = "saturation-threshold",
        value_name = "PERCENT",
        default_value = "20"
    )]
    pub sat_threshold: String,
    #[arg(
        short = 'n',
        long = "num-colors",
        value_name = "NUM",
        default_value = "8"
    )]
    pub num_colors: String,
    #[arg(
        short = 'p',
        long = "sample-fraction",
        value_name = "PERCENT",
        default_value = "5"
    )]
    pub sample_fraction: String,
    #[arg(short = 'w', action = clap::ArgAction::SetTrue)]
    pub white_bg: bool,
    #[arg(short = 'S', action = clap::ArgAction::SetFalse, default_value = "true")]
    pub saturate: bool,
    #[arg(short = 'r', action = clap::ArgAction::SetTrue, default_value = "false")]
    pub return_palette: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            filenames: Vec::new(),
            quiet: false,
            basename: "page".to_string(),
            pdfname: "output.pdf".to_string(),
            dpi: None,
            value_threshold: "25".to_string(),
            sat_threshold: "20".to_string(),
            num_colors: "8".to_string(),
            sample_fraction: "5".to_string(),
            white_bg: false,
            saturate: true,
            return_palette: false,
        }
    }
}

pub fn parse_arguments() -> Options {
    Options::parse()
}
