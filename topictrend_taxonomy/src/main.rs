use anyhow::Result;
use clap::{Parser, Subcommand};
use topictrend_taxonomy::{get_connection, injest, search};

/// CLI for TopicTrend Taxonomy
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index records for a given wiki
    Index {
        /// Wiki name (e.g., enwiki)
        wiki: String,
    },
    /// Search for a query in a given wiki
    Search {
        /// Wiki name (e.g., enwiki)
        wiki: String,
        /// Query string to search
        query: String,
        /// Number of results
        #[clap(default_value_t = 10u64)]
        n: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up better error reporting

    if let Err(e) = run().await {
        eprintln!("Error: {}", e);

        // Print the error chain
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("  Caused by: {}", err);
            source = err.source();
        }

        std::process::exit(1);
    }

    Ok(())
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index { wiki } => {
            println!("Connecting to Qdrant...");
            let client = get_connection()
                .await
                .expect("Failed to connect to Qdrant server");

            println!("Starting indexing for wiki: {}", wiki);
            injest(&client, wiki.clone())
                .await
                .expect(format!("Failed to index wiki '{}'", wiki).as_str());

            println!("✓ Indexing completed successfully for '{}'", wiki);
        }

        Commands::Search { wiki, query, n } => {
            println!("Searching in '{}' for: '{}'", wiki, query);

            let results = search(query.clone(), wiki.clone(), n)
                .await
                .expect(format!("Failed to search in wiki '{}'", wiki).as_str());

            if results.is_empty() {
                println!("No results found for query: '{}'", query);
                return Ok(());
            }

            println!("\n✓ Found {} result(s):\n", results.len());

            for (idx, result) in results.iter().enumerate() {
                println!("Result {}:", idx + 1);
                print!("{}", result);
                println!();
            }
        }
    }

    Ok(())
}
