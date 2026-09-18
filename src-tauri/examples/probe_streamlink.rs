#[tokio::main]
async fn main() {
    let path = std::env::args().nth(1);
    match stream_gui_rs::streamlink::probe(path.as_deref(), std::time::Duration::from_secs(5)).await
    {
        Ok(result) => println!("{} ({})", result.executable, result.version),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
