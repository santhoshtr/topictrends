use topictrend_taxonomy::init_db;

#[tokio::main]
async fn main() {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <wiki>", args[0]);
        std::process::exit(1);
    }
    let wiki = &args[1];
    let uri = format!("{}/{}/categories-lancedb", data_dir, wiki);
    let db = init_db(uri).await.expect("Connection failed");
}
