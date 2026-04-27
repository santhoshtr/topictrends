use clap::{Arg, Command};
use topictrend::pageview_bin::{generate_bin_dump, get_daily_pageviews};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Per Day Wiki Stats")
        .about("Generates per-day wiki statistics")
        .arg(
            Arg::new("wiki")
                .long("wiki")
                .short('w')
                .help("The wiki ID (e.g., enwiki)")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("year")
                .long("year")
                .short('y')
                .help("The year (e.g., 2025)")
                .required(true)
                .value_parser(clap::value_parser!(i16)),
        )
        .arg(
            Arg::new("month")
                .long("month")
                .short('m')
                .help("The month (e.g., 11)")
                .required(true)
                .value_parser(clap::value_parser!(i8)),
        )
        .arg(
            Arg::new("day")
                .long("day")
                .short('d')
                .help("The day (e.g., 24)")
                .required(true)
                .value_parser(clap::value_parser!(i8)),
        )
        .arg(
            Arg::new("output-file")
                .long("output-file")
                .short('o')
                .help("Output file name for the binary pageviews dump")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        )
        .get_matches();

    let wiki = matches.get_one::<String>("wiki").unwrap();
    let year = *matches.get_one::<i16>("year").unwrap();
    let month = *matches.get_one::<i8>("month").unwrap();
    let day = *matches.get_one::<i8>("day").unwrap();
    let output_path = matches.get_one::<String>("output-file").unwrap();

    println!(
        "Processing stats for wiki: {}, date: {}-{}-{}",
        wiki, year, month, day
    );
    let views = get_daily_pageviews(wiki, year, month, day);
    generate_bin_dump(views, output_path)
}
