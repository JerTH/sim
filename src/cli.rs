extern crate clap;
use clap::{Arg, SubCommand};

pub fn parse_command_line() -> clap::ArgMatches<'static> {
    let output_targ_option = Arg::with_name("target")
        .long("target")
        .short("t")
        .required(true)
        .takes_value(true)
        .possible_values(&["console", "file"]);
    
    let output_frequency_option = Arg::with_name("frequency")
        .long("frequency")
        .short("f")
        .required(false)
        .min_values(1);

    let track_targ_option = Arg::with_name("target")
        .long("target")
        .short("t")
        .required(true)
        .min_values(1);

    let track_subcommand = SubCommand::with_name("track")
        .arg(Arg::with_name("kineticenergy").long("kineticenergy").short("k"))
        .arg(Arg::with_name("position").long("position").short("p"))
        .arg(Arg::with_name("velocity").long("velocity").short("v"))
        .arg(Arg::with_name("acceleration").long("acceleration").short("a"))
        .arg(track_targ_option);

    let output_subcommand = SubCommand::with_name("output")
        .arg(output_targ_option)
        .arg(output_frequency_option)
        .arg(Arg::with_name("totalenergy").long("totalenergy").short("e"))
        .arg(Arg::with_name("kineticenergy").long("kineticenergy").short("k"))
        .arg(Arg::with_name("potentialenergy").long("potentialenergy").short("p"))
        .arg(Arg::with_name("frames").long("frames").short("f"))
        .arg(Arg::with_name("memoryuse").long("memuse").short("m"))
        .subcommand(track_subcommand);
    
    let maxsimtime_option = Arg::with_name("maxsimtime")
        .long("maxsimtime")
        .short("t")
        .required(false)
        .takes_value(true);
    
    let maxrealtime_option = Arg::with_name("maxrealtime")
        .long("maxrealtime")
        .short("r")
        .required(false)
        .takes_value(true);

    let timestep_option = Arg::with_name("timestep")
        .long("timestep")
        .short("d")
        .required(false)
        .takes_value(true);

    let simparams_subcommand = SubCommand::with_name("simparams")
        .arg(timestep_option)
        .arg(maxsimtime_option)
        .arg(maxrealtime_option)
        .subcommand(output_subcommand);
    
    clap::App::new("ssim").version("1.0").author("Jeremy T. Hatcher").subcommand(simparams_subcommand).get_matches()
}
