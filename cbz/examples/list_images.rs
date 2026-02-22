use cbz::CbzArchive;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <cbz_file>", args[0]);
        std::process::exit(1);
    }

    let path = &args[1];
    let mut archive = CbzArchive::from_file(path).await?;

    println!("Archive contains {} entries", archive.len());
    println!("\nImages found:");

    for name in archive.image_names() {
        println!("  - {}", name);
    }

    println!("\nReading images:");
    for image in archive.iter_images() {
        match image {
            Ok(img) => {
                println!(
                    "  {} - {} ({} bytes)",
                    img.name,
                    img.mime_type(),
                    img.data.len()
                );
            }
            Err(e) => eprintln!("  Error: {}", e),
        }
    }

    Ok(())
}
