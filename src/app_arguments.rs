use clap::{
    Arg, 
    App
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
}

#[derive(Debug)]
pub enum Action {
    Download(DownloadArguments),
    List
}

#[derive(Debug)]
pub enum VerboseLevel{
    None,
    Medium,
    Max
}

#[derive(Debug)]
pub struct AppArguments {
    pub input: String,
    pub verbose: VerboseLevel,
    pub action: Action
}

pub fn parse_arguments() -> AppArguments {
    let matches = App::new("M3U8 downloader")
        .version("0.5.0")
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
            .help("Stream quality setup (max default): max / select / <quality_index>")
            .takes_value(true))
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .takes_value(true)
            .default_value("0")
            .possible_values(&[
                "0",
                "1",
                "2"
            ])
            .help("Print additional verbose information"))            
        .get_matches();

    let input = matches.value_of("input")
        .map(|v| v.to_owned())
        .expect("Input is missing");        
    let verbose = match matches.value_of("verbose").unwrap() {
        "0" => VerboseLevel::None,
        "1" => VerboseLevel::Medium,
        "2" => VerboseLevel::Max,
        _ => {
            panic!("Invalid verbose level");
        }
    };
    
    if matches.is_present("list_of_specific_quality") {
        return AppArguments{
            input,
            verbose,
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
    let stream_quality_value = match matches.value_of("quality") {
        Some(val) =>{
            match val {
                "max" => StreamQuality::Maximum,
                "select" => StreamQuality::Select,
                val => {
                    let index = val.parse::<u8>()
                        .expect("Stream quality value must be integer");
                    StreamQuality::Specific(index)
                }
            }
        },
        None => {
            StreamQuality::Maximum
        }
    };

    AppArguments{
        input,
        verbose,
        action: Action::Download(DownloadArguments{
            mpv,
            output_file,
            stream_quality_value
        })
    }
}