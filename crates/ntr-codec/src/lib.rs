use image::ImageDecoder;
use image::codecs::jpeg::JpegDecoder;
use std::io::Cursor;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("invalid JPEG frame: {0}")]
    Jpeg(#[from] image::ImageError),
    #[error("decoded frame is too large: {0} bytes")]
    TooLarge(u64),
    #[error("invalid uncompressed frame size: got {actual}, expected {expected}")]
    UncompressedSize { actual: usize, expected: usize },
    #[error("{0} decoding is not implemented in the pure-Rust codec yet")]
    NotImplemented(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn decode_jpeg(encoded: &[u8]) -> Result<DecodedImage, CodecError> {
    let decoder = JpegDecoder::new(Cursor::new(encoded))?;
    let (width, height) = decoder.dimensions();
    let bytes = decoder.total_bytes();
    if bytes > 8 * 1024 * 1024 {
        return Err(CodecError::TooLarge(bytes));
    }
    let color = decoder.color_type();
    let mut pixels = vec![0; bytes as usize];
    decoder.read_image(&mut pixels)?;
    let rgba = match color {
        image::ColorType::Rgb8 => pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        image::ColorType::L8 => pixels.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        image::ColorType::Rgba8 => pixels,
        _ => return Err(CodecError::NotImplemented("JPEG color type")),
    };
    Ok(DecodedImage {
        width,
        height,
        rgba,
    })
}

pub fn decode_uncompressed_rgb(
    encoded: &[u8],
    width: u32,
    height: u32,
) -> Result<DecodedImage, CodecError> {
    let expected = width as usize * height as usize * 3;
    if encoded.len() != expected {
        return Err(CodecError::UncompressedSize {
            actual: encoded.len(),
            expected,
        });
    }
    let rgba = encoded
        .chunks_exact(3)
        .flat_map(|p| [p[0], p[1], p[2], 255])
        .collect();
    Ok(DecodedImage {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_uncompressed_rgb() {
        let image = decode_uncompressed_rgb(&[1, 2, 3, 4, 5, 6], 2, 1).unwrap();
        assert_eq!(image.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn rejects_wrong_uncompressed_size() {
        assert!(decode_uncompressed_rgb(&[0; 5], 2, 1).is_err());
    }
}
