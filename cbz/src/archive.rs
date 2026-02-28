use crate::error::{CbzError, Result};
use std::io::{self, Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

pub struct CbzArchive<R: Read + io::Seek> {
    archive: ZipArchive<R>,
}

impl CbzArchive<Cursor<Vec<u8>>> {
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        let cursor = Cursor::new(data);
        let archive = ZipArchive::new(cursor)?;
        Ok(Self { archive })
    }

    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let data = tokio::fs::read(path.as_ref()).await?;
        Self::from_bytes(data)
    }
}

impl<R: Read + io::Seek> CbzArchive<R> {
    pub fn from_reader(reader: R) -> Result<Self> {
        let archive = ZipArchive::new(reader)?;
        Ok(Self { archive })
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.archive.is_empty()
    }

    pub fn file_names(&self) -> Vec<String> {
        (0..self.archive.len())
            .filter_map(|i| self.archive.name_for_index(i).map(|s| s.to_string()))
            .collect()
    }

    pub fn image_names(&self) -> Vec<String> {
        self.file_names()
            .into_iter()
            .filter(|name| {
                let lower = name.to_lowercase();
                lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".png")
                    || lower.ends_with(".gif")
                    || lower.ends_with(".webp")
                    || lower.ends_with(".bmp")
            })
            .collect()
    }

    pub fn read_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut file = self.archive.by_name(name)?;
        let mut buffer = Vec::with_capacity(file.size() as usize);
        io::copy(&mut file, &mut buffer)?;
        Ok(buffer)
    }

    pub fn read_image(&mut self, name: &str) -> Result<ImageEntry> {
        let data = self.read_entry(name)?;
        let format = detect_image_format(&data)?;
        Ok(ImageEntry {
            name: name.to_string(),
            data,
            format,
        })
    }

    pub fn iter_images(&mut self) -> impl Iterator<Item = Result<ImageEntry>> + '_ {
        let names = self.image_names();
        names.into_iter().filter_map(move |name| {
            let data = self.read_entry(&name).ok()?;
            let format = detect_image_format(&data).ok()?;
            Some(Ok(ImageEntry { name, data, format }))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Bmp,
    Unknown,
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFormat::Jpeg => write!(f, "image/jpeg"),
            ImageFormat::Png => write!(f, "image/png"),
            ImageFormat::Gif => write!(f, "image/gif"),
            ImageFormat::WebP => write!(f, "image/webp"),
            ImageFormat::Bmp => write!(f, "image/bmp"),
            ImageFormat::Unknown => write!(f, "application/octet-stream"),
        }
    }
}

fn detect_image_format(data: &[u8]) -> Result<ImageFormat> {
    if data.len() < 4 {
        return Err(CbzError::InvalidImageFormat);
    }

    if data[0..2] == [0xFF, 0xD8] {
        return Ok(ImageFormat::Jpeg);
    }

    if data[0..4] == [0x89, b'P', b'N', b'G'] {
        return Ok(ImageFormat::Png);
    }

    if data[0..4] == [b'G', b'I', b'F', b'8'] {
        return Ok(ImageFormat::Gif);
    }

    if data[0..4] == [b'R', b'I', b'F', b'F'] && data.len() >= 12 && &data[8..12] == b"WEBP" {
        return Ok(ImageFormat::WebP);
    }

    if data[0..2] == [b'B', b'M'] {
        return Ok(ImageFormat::Bmp);
    }

    Ok(ImageFormat::Unknown)
}

#[derive(Debug)]
pub struct ImageEntry {
    pub name: String,
    pub data: Vec<u8>,
    pub format: ImageFormat,
}

impl ImageEntry {
    pub fn mime_type(&self) -> &'static str {
        match self.format {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Gif => "image/gif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::Unknown => "application/octet-stream",
        }
    }

    pub fn into_stream(self) -> impl futures::Stream<Item = io::Result<Vec<u8>>> {
        futures::stream::once(async move { Ok(self.data) })
    }
}

pub struct ImageStream<R: Read + io::Seek> {
    archive: CbzArchive<R>,
    names: Vec<String>,
    index: usize,
}

impl<R: Read + io::Seek> ImageStream<R> {
    pub fn new(archive: CbzArchive<R>) -> Self {
        let names = archive.image_names();
        Self {
            archive,
            names,
            index: 0,
        }
    }
}

impl<R: Read + io::Seek> Iterator for ImageStream<R> {
    type Item = Result<ImageEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.names.len() {
            return None;
        }
        let name = &self.names[self.index];
        self.index += 1;
        Some(self.archive.read_image(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn create_test_cbz() -> Vec<u8> {
        let mut buffer = Vec::new();
        let cursor = Cursor::new(&mut buffer);
        let mut writer = ZipWriter::new(cursor);

        let options = SimpleFileOptions::default();

        let png_header: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        writer.start_file("page1.png", options).unwrap();
        writer.write_all(&png_header).unwrap();

        writer.start_file("page2.jpg", options).unwrap();
        writer.write_all(&[0xFF, 0xD8, 0xFF, 0xE0]).unwrap();

        writer.start_file("readme.txt", options).unwrap();
        writer.write_all(b"This is not an image").unwrap();

        writer.finish().unwrap();
        buffer
    }

    #[test]
    fn test_from_bytes() {
        let data = create_test_cbz();
        let archive = CbzArchive::from_bytes(data).unwrap();
        assert_eq!(archive.len(), 3);
    }

    #[test]
    fn test_image_names() {
        let data = create_test_cbz();
        let archive = CbzArchive::from_bytes(data).unwrap();
        let images = archive.image_names();
        assert_eq!(images.len(), 2);
        assert!(images.contains(&"page1.png".to_string()));
        assert!(images.contains(&"page2.jpg".to_string()));
    }

    #[test]
    fn test_read_image() {
        let data = create_test_cbz();
        let mut archive = CbzArchive::from_bytes(data).unwrap();

        let image = archive.read_image("page1.png").unwrap();
        assert_eq!(image.name, "page1.png");
        assert_eq!(image.format, ImageFormat::Png);
    }

    #[test]
    fn test_iter_images() {
        let data = create_test_cbz();
        let mut archive = CbzArchive::from_bytes(data).unwrap();

        let images: Vec<_> = archive.iter_images().collect();
        assert_eq!(images.len(), 2);
    }

    #[test]
    fn test_mime_type() {
        let data = create_test_cbz();
        let mut archive = CbzArchive::from_bytes(data).unwrap();

        let png = archive.read_image("page1.png").unwrap();
        assert_eq!(png.mime_type(), "image/png");

        let jpg = archive.read_image("page2.jpg").unwrap();
        assert_eq!(jpg.mime_type(), "image/jpeg");
    }
}
