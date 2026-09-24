//! Title pictures uploaded through the web editor.
//!
//! A phone photo is several megabytes at 4000 px or more, while the recipe
//! page and the listing cards never show one wider than a laptop screen — and
//! the listing loads every card's picture at full size. So an upload is
//! decoded, turned upright, scaled down to [`MAX_EDGE`] and stored as JPEG.
//!
//! JPEG, always: `Recipe.jpg` is the first name `cooklang-find` looks for, so
//! nothing older can hide it, and it is a format cook.md sync carries (it
//! skips `.webp`). A picture with transparency is laid over white first.

use axum::body::Bytes;
use image::{
    codecs::jpeg::JpegEncoder, imageops::FilterType, metadata::Orientation, DynamicImage,
    ImageDecoder, ImageError, ImageFormat, ImageReader, Limits, Rgb, RgbImage,
};
use std::io::Cursor;
use tokio::sync::Semaphore;

/// Largest request body `PUT /api/recipe_image/{*path}` accepts. A
/// full-resolution 48–50 MP phone JPEG is 15–25 MB.
pub const MAX_UPLOAD_BYTES: usize = 40 * 1024 * 1024;

/// Longest edge, in pixels, of a stored title picture.
pub const MAX_EDGE: u32 = 2048;

const JPEG_QUALITY: u8 = 85;

/// Decoding a 50 MP photo holds a few hundred megabytes for a second or two.
/// One at a time keeps a small host — a Raspberry Pi, a phone under Termux —
/// from running out of memory when several arrive together.
static PROCESSING: Semaphore = Semaphore::const_new(1);

#[derive(Debug)]
pub enum PrepareError {
    /// HEIC/HEIF or AVIF — what an iPhone saves by default. Both need a
    /// native C decoder, which the server does not carry.
    Heif,
    /// Not a picture, or not one of JPEG, PNG and WebP.
    Unsupported,
    /// Says it is JPEG, PNG or WebP but does not decode.
    Invalid(String),
    /// Decodes to more pixels than the decoder's memory limit allows.
    TooLarge,
}

impl std::fmt::Display for PrepareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrepareError::Heif => f.write_str(
                "HEIC and AVIF photos can't be read. Upload from the phone's own browser, \
                 which converts them to JPEG, or set the iPhone camera to Most Compatible \
                 (Settings, Camera, Formats).",
            ),
            PrepareError::Unsupported => {
                f.write_str("Only JPEG, PNG and WebP pictures are supported.")
            }
            PrepareError::Invalid(e) => write!(f, "The picture could not be read: {e}"),
            PrepareError::TooLarge => f.write_str("The picture has too many pixels to process."),
        }
    }
}

/// [`prepare`] off the async runtime, one picture at a time.
pub async fn prepare_async(bytes: Bytes) -> Result<Vec<u8>, PrepareError> {
    let permit = PROCESSING
        .acquire()
        .await
        .expect("the processing semaphore is never closed");
    tokio::task::spawn_blocking(move || {
        // Held by the blocking task rather than the request: a client that
        // hangs up drops the request, not the decode already running.
        let _permit = permit;
        prepare(&bytes)
    })
    .await
    .map_err(|e| PrepareError::Invalid(e.to_string()))?
}

/// Turns an uploaded picture into the JPEG bytes to store.
pub fn prepare(bytes: &[u8]) -> Result<Vec<u8>, PrepareError> {
    if is_heif(bytes) {
        return Err(PrepareError::Heif);
    }
    let format = image::guess_format(bytes).map_err(|_| PrepareError::Unsupported)?;
    if !matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP
    ) {
        return Err(PrepareError::Unsupported);
    }

    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(Limits::default());
    let mut decoder = reader.into_decoder().map_err(decode_error)?;
    // A photo is stored sensor-side up with an Exif note saying how to turn
    // it. Re-encoding drops the note, so the turn has to be applied here.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let image = DynamicImage::from_decoder(decoder).map_err(decode_error)?;

    let fits = image.width() <= MAX_EDGE && image.height() <= MAX_EDGE;
    let mut picture = DynamicImage::ImageRgb8(flatten(image));
    if !fits {
        picture = picture.resize(MAX_EDGE, MAX_EDGE, FilterType::CatmullRom);
    }
    // After the resize, which is cheaper on the smaller image: the bound is
    // square, so fitting then turning lands on the same size as the reverse.
    picture.apply_orientation(orientation);
    let encoded = encode_jpeg(&picture.into_rgb8())?;

    // A JPEG that needed nothing done would only lose quality to a second
    // encode, and if it is already the smaller of the two it is kept as sent.
    if format == ImageFormat::Jpeg
        && orientation == Orientation::NoTransforms
        && fits
        && bytes.len() <= encoded.len()
    {
        return Ok(bytes.to_vec());
    }
    Ok(encoded)
}

/// Whether `bytes` open an ISO-BMFF `ftyp` box naming a HEIF or AVIF brand.
fn is_heif(bytes: &[u8]) -> bool {
    const BRANDS: [&[u8; 4]; 10] = [
        b"heic", b"heix", b"hevc", b"hevx", b"heim", b"heis", b"mif1", b"msf1", b"avif", b"avis",
    ];
    bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && BRANDS.iter().any(|b| &bytes[8..12] == *b)
}

fn decode_error(e: ImageError) -> PrepareError {
    match e {
        ImageError::Limits(_) => PrepareError::TooLarge,
        e => PrepareError::Invalid(e.to_string()),
    }
}

/// Drops the alpha channel by laying the picture over white. A plain
/// `to_rgb8` keeps whatever colour sits under a transparent pixel, which is
/// usually black.
fn flatten(image: DynamicImage) -> RgbImage {
    if !image.color().has_alpha() {
        return image.into_rgb8();
    }
    let rgba = image.into_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let a = u32::from(a);
        let over_white = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
        Rgb([over_white(r), over_white(g), over_white(b)])
    })
}

fn encode_jpeg(picture: &RgbImage) -> Result<Vec<u8>, PrepareError> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(picture)
        .map_err(|e| PrepareError::Invalid(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, Rgba, RgbaImage};

    /// A gradient rather than a flat colour, so JPEG sizes depend on quality.
    fn gradient(width: u32, height: u32) -> RgbImage {
        RgbImage::from_fn(width, height, |x, y| {
            Rgb([
                (x * 7 % 256) as u8,
                (y * 5 % 256) as u8,
                ((x + y) % 256) as u8,
            ])
        })
    }

    fn encode(picture: DynamicImage, format: ImageFormat) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        picture.write_to(&mut out, format).unwrap();
        out.into_inner()
    }

    fn jpeg(picture: &RgbImage, quality: u8, exif: Option<Vec<u8>>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut out, quality);
        if let Some(exif) = exif {
            encoder.set_exif_metadata(exif).unwrap();
        }
        encoder.encode_image(picture).unwrap();
        out
    }

    /// A big-endian TIFF block holding a single Orientation entry.
    fn exif_orientation(value: u16) -> Vec<u8> {
        let mut exif = b"MM\0\x2a\0\0\0\x08".to_vec();
        exif.extend_from_slice(&1u16.to_be_bytes());
        exif.extend_from_slice(&0x0112u16.to_be_bytes());
        exif.extend_from_slice(&3u16.to_be_bytes());
        exif.extend_from_slice(&1u32.to_be_bytes());
        exif.extend_from_slice(&value.to_be_bytes());
        exif.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        exif
    }

    fn decoded(bytes: &[u8]) -> DynamicImage {
        assert_eq!(image::guess_format(bytes).unwrap(), ImageFormat::Jpeg);
        image::load_from_memory(bytes).unwrap()
    }

    #[test]
    fn an_oversized_picture_is_scaled_to_the_max_edge() {
        let input = jpeg(&gradient(3000, 1500), 90, None);
        let out = decoded(&prepare(&input).unwrap());
        assert_eq!((out.width(), out.height()), (MAX_EDGE, MAX_EDGE / 2));
    }

    #[test]
    fn a_png_is_stored_as_jpeg() {
        let input = encode(DynamicImage::ImageRgb8(gradient(64, 32)), ImageFormat::Png);
        let out = decoded(&prepare(&input).unwrap());
        assert_eq!((out.width(), out.height()), (64, 32));
    }

    #[test]
    fn a_webp_is_stored_as_jpeg() {
        let input = encode(DynamicImage::ImageRgb8(gradient(64, 32)), ImageFormat::WebP);
        let out = decoded(&prepare(&input).unwrap());
        assert_eq!((out.width(), out.height()), (64, 32));
    }

    #[test]
    fn transparency_is_laid_over_white() {
        let clear = RgbaImage::from_pixel(16, 16, Rgba([0, 0, 0, 0]));
        let input = encode(DynamicImage::ImageRgba8(clear), ImageFormat::Png);
        let out = decoded(&prepare(&input).unwrap()).into_rgb8();
        for pixel in out.pixels() {
            assert!(
                pixel.0.iter().all(|&c| c >= 250),
                "expected white, got {pixel:?}"
            );
        }
    }

    #[test]
    fn a_small_well_compressed_jpeg_is_kept_as_sent() {
        let input = jpeg(&gradient(64, 64), 40, None);
        assert_eq!(prepare(&input).unwrap(), input);
    }

    #[test]
    fn the_exif_orientation_is_applied() {
        // 6 is "rotate 90° clockwise to display".
        let input = jpeg(&gradient(40, 20), 40, Some(exif_orientation(6)));
        let out = prepare(&input).unwrap();
        assert_ne!(out, input, "a turned picture must be re-encoded");
        let out = decoded(&out);
        assert_eq!((out.width(), out.height()), (20, 40));
    }

    #[test]
    fn heic_and_avif_are_recognised() {
        for brand in [b"heic", b"mif1", b"avif"] {
            let mut input = b"\0\0\0\x18ftyp".to_vec();
            input.extend_from_slice(brand);
            input.extend_from_slice(b"\0\0\0\0mif1heic");
            assert!(
                matches!(prepare(&input), Err(PrepareError::Heif)),
                "{} must be refused as HEIF",
                String::from_utf8_lossy(brand)
            );
        }
    }

    #[test]
    fn other_formats_are_unsupported() {
        for input in [&b"GIF89a\x01\0\x01\0\0\0\0;"[..], b"just some text", b""] {
            assert!(matches!(prepare(input), Err(PrepareError::Unsupported)));
        }
    }

    #[test]
    fn a_truncated_jpeg_is_invalid() {
        let input = jpeg(&gradient(64, 64), 90, None);
        assert!(matches!(
            prepare(&input[..20]),
            Err(PrepareError::Invalid(_))
        ));
    }
}
