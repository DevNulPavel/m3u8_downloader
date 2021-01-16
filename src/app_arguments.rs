use clap::{
    Arg, 
    App,
    SubCommand
};

#[derive(Debug)]
pub enum StreamQuality {
    Maximum,
    Specific(u8),
    Select
}

#[derive(Debug)]
pub struct DownloadArguments {
    pub mpv: bool,
    pub output_file: Option<String>,
    pub stream_quality_value: StreamQuality
    // TODO: Verbose
}

#[derive(Debug)]
pub enum Action {
    Download(DownloadArguments),
    List
}

#[derive(Debug)]
pub struct AppArguments {
    pub input: String,
    pub action: Action
}

pub fn parse_arguments() -> AppArguments {
    let matches = App::new("M3U8 downloader")
        .version("1.0.0")
        .author("DevNul <devnulpavel@gmail.com>")
        .about("Allow you download HLS m3u8 streams")
        .arg(Arg::with_name("list_of_specific_quality")
            .short("l")
            .long("list_of_specific_quality")
            .next_line_help(true)
            .conflicts_with_all(&[ // TODO: Не нашлось другого способа оставить лишь его
                "output",
                "mpv",
                "quality",
                "specific_quality_value"
            ])
            .help("Request only possible specific quality values for <specific_quality_value> option"))
        .arg(Arg::with_name("input")
                .help("Input M3U8 playlist url")
                .required(true))
        .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .next_line_help(true)
                .help("Output file path, \
                        '<datetime>.ts' in current folder \
                        will be used as default if not provided"))
        .arg(Arg::with_name("mpv")
                .short("m")
                .long("mpv")
                .next_line_help(true)
                .help("Use MPV for stream monotoring"))
        .arg(Arg::with_name("quality")
            .short("q")
            .long("quality")
            .next_line_help(true)
            .default_value("maximum")
            .possible_values(&[
                "select",
                "set",
                "maximum"
            ])
            .help("Stream quality setup")
            .takes_value(true))
        .arg(Arg::with_name("set_quality_value")
            .short("s")
            .long("set_quality_value")
            .help("Stream quality value if 'set' value present")
            .takes_value(true)
            .next_line_help(true)
            .requires_if("quality", "set"))
        .get_matches();

    let input = matches.value_of("input")
        .map(|v| v.to_owned())
        .expect("Input is missing");        
    
    if matches.is_present("list_of_specific_quality") {
        return AppArguments{
            input,
            action: Action::List
        };
    }        

    let input = matches.value_of("input")
        .map(|v| v.to_owned())
        .expect("Input is missing");
    let output_file = matches
        .value_of("output")
        .map(|v| v.to_owned());
    let mpv = matches
        .is_present("mpv");
    let stream_quality_value = match matches.value_of("quality").unwrap() {
        "maximum" => StreamQuality::Maximum,
        "select" => StreamQuality::Select,
        "set" => {
            let quality = matches
                .value_of("set_quality_value")
                .expect("set_quality_value must be present")
                .parse::<u8>()
                .expect("Stream quality value must be number value between <0-255>");
            StreamQuality::Specific(quality)
        },
        _ => {
            panic!("Unspecified quality value");
        }
    };

    AppArguments{
        input,
        action: Action::Download(DownloadArguments{
            mpv,
            output_file,
            stream_quality_value
        })
    }
}