use std::path::PathBuf;

use clap::Parser;
use comic_ocr::manga_ocr::MangaOcr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the image file
    #[arg(short, long)]
    image: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if !args.image.exists() {
        anyhow::bail!("Image file not found: {:?}", args.image);
    }

    println!("Loading OCR model...");
    let ocr = MangaOcr::load(false).await?;

    println!("Processing image: {:?}", args.image);
    let image = image::open(&args.image)?;
    
    // Resize to the expected input size for the OCR model
    let resized = image.resize_exact(224, 224, image::imageops::FilterType::Triangle);
    
    println!("Running OCR...");
    let texts = ocr.inference(&[resized])?;
    
    println!("OCR Result: {:?}", texts);
    
    if !texts.is_empty() {
        println!("\nDetected text: {}", texts[0]);
    }

    Ok(())
}
