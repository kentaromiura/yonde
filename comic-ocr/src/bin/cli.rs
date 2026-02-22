use std::path::PathBuf;

use clap::Parser;
use comic_ocr::{comic_text_detector::ComicTextDetector, manga_ocr::MangaOcr};
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the image file
    #[arg(short, long)]
    image: PathBuf,

    /// Use CPU instead of GPU
    #[arg(long, default_value_t = false)]
    cpu: bool,
}

#[derive(Serialize)]
struct TextRegion {
    text: String,
    box_2d: [usize; 4], // [xmin, ymin, width, height]
    confidence: f32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();

    let args = Args::parse();

    if !args.image.exists() {
        anyhow::bail!("Image file not found: {:?}", args.image);
    }

    let detector = ComicTextDetector::load(args.cpu).await?;
    let ocr = MangaOcr::load(args.cpu).await?;

    let image = image::open(&args.image)?;
    
    let bboxes = detector.inference(&image)?;

    let mut crops = Vec::new();
    let mut regions = Vec::new();

    for bbox in &bboxes {
        let width = bbox.xmax - bbox.xmin;
        let height = bbox.ymax - bbox.ymin;
        
        // Use crop_imm to create an actual copy of the region
        let crop = image.crop_imm(
            bbox.xmin as u32, 
            bbox.ymin as u32, 
            width as u32, 
            height as u32
        );
        
        crops.push(crop);
    }

    if !crops.is_empty() {
        let texts = ocr.inference(&crops)?;

        for (bbox, text) in bboxes.iter().zip(texts.into_iter()) {
             let width = bbox.xmax - bbox.xmin;
             let height = bbox.ymax - bbox.ymin;
             
             regions.push(TextRegion {
                 text,
                 box_2d: [bbox.xmin as usize, bbox.ymin as usize, width as usize, height as usize],
                 confidence: bbox.confidence,
             });
        }
    }

    let json = serde_json::to_string_pretty(&regions)?;
    println!("{}", json);

    Ok(())
}
