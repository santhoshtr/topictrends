use clap::{Arg, Command};
use polars::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("get-gsc-qid-date")
        .about("Maps Google Search Console page data to QIDs for a specific wiki and date")
        .arg(
            Arg::new("wiki")
                .long("wiki")
                .short('w')
                .required(true)
                .help("Wiki ID (e.g. enwiki, eswiki)"),
        )
        .arg(
            Arg::new("date")
                .long("date")
                .short('d')
                .required(true)
                .help("Date to process (YYYY-MM-DD)"),
        )
        .arg(
            Arg::new("gsc-dir")
                .long("gsc-dir")
                .default_value("data/gsc_page_date")
                .help("Path to GSC source root (Hive-partitioned by date=YYYY-MM-DD)"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .required(true)
                .help("Output parquet path"),
        )
        .get_matches();

    let wiki = matches.get_one::<String>("wiki").unwrap();
    let date = matches.get_one::<String>("date").unwrap();
    let gsc_dir = matches.get_one::<String>("gsc-dir").unwrap();
    let output = matches.get_one::<String>("output").unwrap();

    process(wiki, date, gsc_dir, output)
}

/// Extract (lang, raw_title) from a Wikipedia article URL.
/// Returns None for non-article URLs (portal, mobile, variant paths, etc.).
fn parse_url(url: &str) -> Option<(&str, &str)> {
    // Must be https://<lang>.wikipedia.org/wiki/<title>
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;

    // Must be <lang>.wikipedia.org — reject www, en.m, zh (no subdomain match), etc.
    let lang = host.strip_suffix(".wikipedia.org")?;

    // lang must not contain dots (rules out en.m) and must not be "www"
    if lang.contains('.') || lang == "www" {
        return None;
    }

    // Path must start with wiki/
    let title = path.strip_prefix("wiki/")?;

    // Title must be non-empty and must not contain '/' (no subpages here)
    if title.is_empty() {
        return None;
    }

    Some((lang, title))
}

/// Percent-decode a URL title component.
/// Leaves underscores intact — articles.parquet uses underscore-spaced titles.
/// Collects raw bytes first, then interprets as UTF-8, so multi-byte sequences
/// like %C3%93 (Ó) are handled correctly.
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut buf: Vec<u8> = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                buf.push(((h << 4) | l) as u8);
                i += 3;
                continue;
            }
        }
        buf.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Load articles.parquet -> HashMap<title, qid>
fn load_title_to_qid(wiki: &str) -> Result<HashMap<String, u32>, Box<dyn std::error::Error>> {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let path_str = format!("{}/{}/articles.parquet", data_dir, wiki);

    println!("Loading articles from: {}", path_str);

    let path = PlRefPath::try_from_path(Path::new(&path_str))?;
    let df = LazyFrame::scan_parquet(path, Default::default())
        .map_err(|e| format!("Cannot open {}: {}", path_str, e))?
        .select([col("page_title"), col("qid")])
        .collect()?;

    let titles = df.column("page_title")?.str()?;
    let qids = df.column("qid")?.u32()?;

    let mut map = HashMap::with_capacity(df.height());
    for i in 0..df.height() {
        if let (Some(title), Some(qid)) = (titles.get(i), qids.get(i)) {
            map.insert(title.to_string(), qid);
        }
    }

    println!("Loaded {} title→qid mappings for {}", map.len(), wiki);
    Ok(map)
}

fn process(
    wiki: &str,
    date: &str,
    gsc_dir: &str,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Expected wiki lang prefix: "enwiki" -> "en", "zh_classicalwiki" -> "zh_classical" etc.
    // The wiki suffix is always "wiki"; strip it to get the lang code used in URLs.
    let lang_prefix = wiki
        .strip_suffix("wiki")
        .ok_or_else(|| format!("Wiki ID must end with 'wiki', got: {}", wiki))?;

    let gsc_parquet = format!("{}/date={}/data.parquet", gsc_dir, date);
    println!("Reading GSC data from: {}", gsc_parquet);

    if !std::path::Path::new(&gsc_parquet).exists() {
        return Err(format!("GSC parquet not found: {}", gsc_parquet).into());
    }

    // Read GSC parquet
    let path = PlRefPath::try_from_path(Path::new(&gsc_parquet))?;
    let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

    let pages = df.column("page")?.str()?;
    let clicks_col = df.column("clicks")?.i64()?;
    let impressions_col = df.column("impressions")?.i64()?;
    let position_col = df.column("position")?.f64()?;

    let total_rows = df.height();

    // Load QID mapping for this wiki
    let title_to_qid = load_title_to_qid(wiki)?;

    // Accumulators: qid -> (clicks, impressions, position_x_impressions)
    let mut agg: HashMap<u32, (i64, i64, f64)> = HashMap::new();
    let mut url_parse_failures = 0usize;
    let mut unmapped_titles = 0usize;
    let mut wrong_wiki = 0usize;

    for i in 0..total_rows {
        let page = match pages.get(i) {
            Some(p) => p,
            None => {
                url_parse_failures += 1;
                continue;
            }
        };

        let (lang, raw_title) = match parse_url(page) {
            Some(t) => t,
            None => {
                url_parse_failures += 1;
                continue;
            }
        };

        if lang != lang_prefix {
            wrong_wiki += 1;
            continue;
        }

        // URL-decode title; articles.parquet keeps underscores
        let title = url_decode(raw_title);

        let qid = match title_to_qid.get(&title) {
            Some(&q) => q,
            None => {
                unmapped_titles += 1;
                continue;
            }
        };

        let clicks = clicks_col.get(i).unwrap_or(0);
        let impressions = impressions_col.get(i).unwrap_or(0);
        let position = position_col.get(i).unwrap_or(0.0);

        let entry = agg.entry(qid).or_insert((0, 0, 0.0));
        entry.0 += clicks;
        entry.1 += impressions;
        entry.2 += position * impressions as f64;
    }

    let output_rows = agg.len();

    println!("Stats:");
    println!("  Total input rows:      {}", total_rows);
    println!("  Wrong wiki (filtered): {}", wrong_wiki);
    println!("  URL parse failures:    {}", url_parse_failures);
    println!("  Unmapped titles:       {}", unmapped_titles);
    println!("  Output rows:           {}", output_rows);

    if output_rows == 0 {
        return Err(format!(
            "No rows mapped for wiki={} date={} — check articles.parquet exists",
            wiki, date
        )
        .into());
    }

    // Build output DataFrame
    let mut qids_vec: Vec<u32> = Vec::with_capacity(output_rows);
    let mut clicks_vec: Vec<i64> = Vec::with_capacity(output_rows);
    let mut impressions_vec: Vec<i64> = Vec::with_capacity(output_rows);
    let mut ctr_vec: Vec<f64> = Vec::with_capacity(output_rows);
    let mut position_vec: Vec<f64> = Vec::with_capacity(output_rows);

    for (qid, (clicks, impressions, pos_x_impr)) in agg {
        qids_vec.push(qid);
        clicks_vec.push(clicks);
        impressions_vec.push(impressions);
        ctr_vec.push(if impressions > 0 {
            clicks as f64 / impressions as f64
        } else {
            0.0
        });
        position_vec.push(if impressions > 0 {
            pos_x_impr / impressions as f64
        } else {
            0.0
        });
    }

    let mut out_df = DataFrame::new(
        output_rows,
        vec![
            Column::new("qid".into(), qids_vec),
            Column::new("clicks".into(), clicks_vec),
            Column::new("impressions".into(), impressions_vec),
            Column::new("ctr".into(), ctr_vec),
            Column::new("position".into(), position_vec),
        ],
    )?;

    // Create output directory
    if let Some(parent) = Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(output)
        .map_err(|e| format!("Failed to create output file '{}': {}", output, e))?;

    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut out_df)
        .map_err(|e| format!("Failed to write parquet '{}': {}", output, e))?;

    file.sync_all()?;

    println!("Written: {}", output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_basic() {
        let (lang, title) = parse_url("https://en.wikipedia.org/wiki/Ali_Khamenei").unwrap();
        assert_eq!(lang, "en");
        assert_eq!(title, "Ali_Khamenei");
    }

    #[test]
    fn test_parse_url_dot_title() {
        let (lang, title) = parse_url("https://en.wikipedia.org/wiki/.xxx").unwrap();
        assert_eq!(lang, "en");
        assert_eq!(title, ".xxx");
    }

    #[test]
    fn test_parse_url_encoded() {
        let (lang, title) = parse_url("https://es.wikipedia.org/wiki/Fernando_%C3%93nega").unwrap();
        assert_eq!(lang, "es");
        assert_eq!(title, "Fernando_%C3%93nega");
    }

    #[test]
    fn test_parse_url_rejects_www() {
        assert!(parse_url("https://www.wikipedia.org/").is_none());
    }

    #[test]
    fn test_parse_url_rejects_mobile() {
        assert!(parse_url("https://en.m.wikipedia.org/wiki/Foo").is_none());
    }

    #[test]
    fn test_parse_url_rejects_variant_path() {
        // zh.wikipedia.org/zh-tw/... — path doesn't start with wiki/
        assert!(parse_url("https://zh.wikipedia.org/zh-tw/%E4%BC%8A%E6%9C%97").is_none());
    }

    #[test]
    fn test_url_decode_plain() {
        assert_eq!(url_decode("Ali_Khamenei"), "Ali_Khamenei");
    }

    #[test]
    fn test_url_decode_percent() {
        assert_eq!(url_decode("Fernando_%C3%93nega"), "Fernando_Ónega");
    }

    #[test]
    fn test_url_decode_preserves_underscore() {
        assert_eq!(url_decode("Some_Article"), "Some_Article");
    }
}
