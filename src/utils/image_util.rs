use anyhow::{Result, anyhow};
use data_url::DataUrl;
use image::{ColorType, DynamicImage, GenericImageView, ImageError, codecs::jpeg::JpegEncoder, imageops::FilterType};

#[derive(Copy, Clone, Debug, Hash)]
pub(crate) enum ImageRotation {
    Rot90,
    Rot270,
}

#[derive(Copy, Clone, Debug, Hash)]
pub(crate) enum ImageMirroring {
    None,
    Both,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum ImageMode {
    JPEG,
}

#[derive(Copy, Clone, Debug, Hash)]
pub(crate) struct ImageFormat {
    pub mode: ImageMode,
    pub size: (u16, u16),
    pub rotation: ImageRotation,
    pub mirror: ImageMirroring,
}

pub(crate) fn decode_image_data_url(url: &str) -> Result<DynamicImage> {
    let data_url = DataUrl::process(url).map_err(|_| anyhow!("failed to parse data URL"))?;
    let (bytes, _) = data_url.decode_to_vec().map_err(|_| anyhow!("failed to decode data URL"))?;

    let format = match (data_url.mime_type().type_.as_str(), data_url.mime_type().subtype.as_str()) {
        (_, "jpeg" | "jpg") => image::ImageFormat::Jpeg,
        (_, "png") => image::ImageFormat::Png,
        _ => {
            return Err(anyhow!("unsupported image mime type: {}", data_url.mime_type()));
        }
    };

    Ok(image::load_from_memory_with_format(&bytes, format)?)
}

pub(crate) fn rotated_dimensions(size: (u16, u16), rotation: ImageRotation) -> (u16, u16) {
    let (w, h) = size;
    match rotation {
        ImageRotation::Rot90 | ImageRotation::Rot270 => (h, w),
    }
}

fn encode_image_impl(image_format: ImageFormat, image: &DynamicImage) -> Result<Vec<u8>, ImageError> {
    let image = transform_image(image_format, image);
    encode_buffer(&image)
}

fn transform_image(image_format: ImageFormat, image: &DynamicImage) -> DynamicImage {
    let (ws, hs) = image_format.size;
    let image = image.resize_exact(u32::from(ws), u32::from(hs), FilterType::Lanczos3);

    let image = match image_format.rotation {
        ImageRotation::Rot90 => image.rotate90(),
        ImageRotation::Rot270 => image.rotate270(),
    };

    match image_format.mirror {
        ImageMirroring::None => image,
        ImageMirroring::Both => image.fliph().flipv(),
    }
}

fn encode_buffer(image: &DynamicImage) -> Result<Vec<u8>, ImageError> {
    let (w, h) = image.dimensions();
    let rgb = image.to_rgb8();
    let image_data = rgb.as_raw();
    let mut buf = Vec::new();
    let quality = if w == 96 && h == 96 { 90 } else { 70 };
    let mut encoder = JpegEncoder::new_with_quality(&mut buf, quality);
    encoder.encode(image_data, w, h, ColorType::Rgb8.into())?;
    Ok(buf)
}

pub(crate) async fn encode_image(image_format: ImageFormat, image: &DynamicImage) -> Result<Vec<u8>, ImageError> {
    tokio::task::block_in_place(|| encode_image_impl(image_format, image))
}
