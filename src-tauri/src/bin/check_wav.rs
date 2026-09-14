use phrasewatch_lib::engine;
use phrasewatch_lib::models::AppConfig;
use std::env;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from(env::args().nth(1).expect("wav path"));
    let mut cfg = AppConfig::default();
    cfg.debounce_seconds = 0.0;
    match engine::process_wav(&path, &cfg) {
        Ok(hits) => {
            if hits.is_empty() {
                println!("No phrase detected.");
                std::process::exit(0);
            }
            for hit in hits {
                println!(
                    "HIT  phrase={:?}  source={}  transcript={:?}",
                    hit.phrase, hit.source, hit.transcript
                );
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}
