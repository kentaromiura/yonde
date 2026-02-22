use comic_ocr::manga_ocr::MangaOcr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("Loading MangaOCR model...");
    let ocr = MangaOcr::load(false).await?;
    
    // Create a simple test image with black text on white background
    // 224x224 image (the expected input size)
    let size = 224u32;
    let mut img = image::RgbaImage::new(size, size);
    
    // Fill with white
    for y in 0..size {
        for x in 0..size {
            img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
        }
    }
    
    // Draw a simple black rectangle to simulate text
    for y in 50..100 {
        for x in 50..150 {
            img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    
    println!("Testing OCR on synthetic image...");
    let dynamic_img = image::DynamicImage::ImageRgba8(img);
    
    let texts = ocr.inference(&[dynamic_img])?;
    
    println!("OCR result: {:?}", texts);
    
    Ok(())
}
