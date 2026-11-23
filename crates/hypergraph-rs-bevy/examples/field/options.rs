//! The example's command line, read without a window so a test can reach it.

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub width: u32,
    pub height: u32,
    pub measure_seconds: Option<f64>,
    pub fullscreen: bool,
    /// The large world, for the measured run; without it the small one.
    pub large: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            width: 1366,
            height: 768,
            measure_seconds: None,
            fullscreen: false,
            large: false,
        }
    }
}

const USAGE: &str = "use --size WxH, --measure-seconds N, --fullscreen or --large";

/// Reads the arguments after the program's name.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--size" => {
                let value = args.next().ok_or("--size needs WxH")?;
                let (w, h) = value.split_once('x').ok_or("--size is WxH, as 1366x768")?;
                options.width = w.parse().map_err(|_| "a width in pixels")?;
                options.height = h.parse().map_err(|_| "a height in pixels")?;
            }
            "--measure-seconds" => {
                let value = args.next().ok_or("--measure-seconds needs a number")?;
                options.measure_seconds = Some(value.parse().map_err(|_| "seconds")?);
            }
            "--fullscreen" => options.fullscreen = true,
            "--large" => options.large = true,
            other => return Err(format!("unknown argument {other}; {USAGE}")),
        }
    }
    Ok(options)
}
