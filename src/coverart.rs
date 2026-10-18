// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Cover art support.

use crate::config::CoverArtResolution;
use crate::util::format_size;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Maximum size of a downloaded cover image in bytes.
pub const MAX_IMAGE_SIZE: u64 = 32 * 1024 * 1024;

/// Image formats that can be embedded as cover art.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG image.
    Jpeg,
    /// PNG image.
    Png,
}

impl ImageFormat {
    /// Detect the image format from the magic bytes at the start of the data.
    #[must_use]
    pub fn sniff(data: &[u8]) -> Option<Self> {
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
            Some(Self::Png)
        } else {
            None
        }
    }

    /// Short human-readable name of the image format.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Jpeg => "JPG",
            Self::Png => "PNG",
        }
    }

    /// The MIME type of the image format.
    #[must_use]
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }
}

/// A cover image that can be embedded into a tag.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverArt {
    /// Format of the image data.
    #[serde(with = "image_format_serde")]
    format: ImageFormat,
    /// The raw (encoded) image data.
    #[serde(with = "base64_serde")]
    data: Vec<u8>,
}

impl std::fmt::Debug for CoverArt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoverArt")
            .field("format", &self.format)
            .field("data", &format_args!("{} bytes", self.data.len()))
            .finish()
    }
}

impl CoverArt {
    /// Create cover art from raw image data, detecting the format.
    ///
    /// Returns `None` if the data is not a supported image format.
    #[must_use]
    pub fn from_data(data: Vec<u8>) -> Option<Self> {
        let format = ImageFormat::sniff(&data)?;
        Some(Self { format, data })
    }

    /// The image format.
    #[must_use]
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// The MIME type of the image.
    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        self.format.mime_type()
    }

    /// A short description, e.g. `Front Cover (JPG, 500x500px, 32KiB)`.
    ///
    /// Falls back to the resolution label if the dimensions can't be determined.
    #[must_use]
    pub fn description(&self, resolution: CoverArtResolution) -> String {
        let dimensions = image_dimensions(&self.data).map_or_else(
            || resolution.label().to_owned(),
            |(width, height)| format!("{width}x{height}px"),
        );
        format!(
            "Front Cover ({}, {dimensions}, {})",
            self.format.name(),
            format_size(u64::try_from(self.data.len()).unwrap_or(u64::MAX))
        )
    }

    /// The raw image data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Read the pixel dimensions `(width, height)` from the header of a PNG or JPEG image.
#[must_use]
pub fn image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    match ImageFormat::sniff(data)? {
        ImageFormat::Png => {
            // 8 byte signature, 4 byte chunk length, "IHDR", then width and height.
            let ihdr = data.get(12..24)?;
            if &ihdr[..4] != b"IHDR" {
                return None;
            }
            let width = u32::from_be_bytes(ihdr[4..8].try_into().ok()?);
            let height = u32::from_be_bytes(ihdr[8..12].try_into().ok()?);
            Some((width, height))
        }
        ImageFormat::Jpeg => {
            let mut pos = 2;
            loop {
                if *data.get(pos)? != 0xFF {
                    return None;
                }
                let marker = *data.get(pos + 1)?;
                match marker {
                    0xFF => pos += 1,
                    // Markers without a length field.
                    0x01 | 0xD0..=0xD7 => pos += 2,
                    // Start of scan or end of image before any frame header.
                    0xD8..=0xDA => return None,
                    // Start of frame (excluding DHT, JPG and DAC markers).
                    0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                        let segment = data.get(pos + 4..pos + 9)?;
                        let height = u32::from(u16::from_be_bytes([segment[1], segment[2]]));
                        let width = u32::from(u16::from_be_bytes([segment[3], segment[4]]));
                        return Some((width, height));
                    }
                    _ => {
                        let length = data.get(pos + 2..pos + 4)?;
                        pos += 2 + usize::from(u16::from_be_bytes([length[0], length[1]]));
                    }
                }
            }
        }
    }
}

/// Hash the image data so that identical images can be detected.
#[must_use]
pub fn fingerprint(data: &[u8]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Information about a cover image that is already embedded in a file.
///
/// The image might be in a format that cannot be embedded by us, so the details are optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverInfo {
    /// Hash of the image data, used to detect identical images.
    pub fingerprint: u64,
    /// Detected image format.
    pub format: Option<ImageFormat>,
    /// Pixel dimensions `(width, height)`.
    pub dimensions: Option<(u32, u32)>,
    /// Size of the image data in bytes.
    pub size: usize,
}

impl CoverInfo {
    /// Gather information about the given image data.
    #[must_use]
    pub fn from_data(data: &[u8]) -> Self {
        Self {
            fingerprint: fingerprint(data),
            format: ImageFormat::sniff(data),
            dimensions: image_dimensions(data),
            size: data.len(),
        }
    }

    /// A short description, e.g. `Front Cover (JPG, 500x500px, 32 KiB)`.
    #[must_use]
    pub fn description(&self) -> String {
        let details = [
            self.format.map(|format| format.name().to_owned()),
            self.dimensions
                .map(|(width, height)| format!("{width}x{height}px")),
            Some(format_size(u64::try_from(self.size).unwrap_or(u64::MAX))),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        format!("Front Cover ({details})")
    }
}

/// Check whether an error (or any of its sources) is a HTTP 404 response.
#[must_use]
pub fn is_not_found(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut current = Some(err);
    while let Some(err) = current {
        if matches!(
            err.downcast_ref::<ureq::Error>(),
            Some(ureq::Error::StatusCode(404))
        ) {
            return true;
        }
        current = err.source();
    }
    false
}

/// Download an image from the given URL (blocking).
///
/// # Errors
///
/// Returns an error if the download fails or the image is not in a supported format.
pub fn download(url: &str, user_agent: &str) -> crate::Result<CoverArt> {
    let mut response = ureq::get(url)
        .header("User-Agent", user_agent)
        .call()
        .map_err(Box::new)?;
    let data = response
        .body_mut()
        .with_config()
        .limit(MAX_IMAGE_SIZE)
        .read_to_vec()
        .map_err(Box::new)?;
    CoverArt::from_data(data).ok_or(crate::Error::UnsupportedCoverArtFormat)
}

/// Serde helper to store the image data as a base64 string (the cache is JSON-based).
mod base64_serde {
    use super::{Deserialize, Deserializer, Engine, Serializer, STANDARD};

    /// Serialize bytes as base64 string.
    pub fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(data))
    }

    /// Deserialize bytes from a base64 string.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}

/// Serde helper to store the image format as MIME type.
mod image_format_serde {
    use super::{Deserialize, Deserializer, ImageFormat, Serializer};

    /// Serialize the format as MIME type.
    #[allow(clippy::trivially_copy_pass_by_ref)] // signature required by serde
    pub fn serialize<S: Serializer>(
        format: &ImageFormat,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(format.mime_type())
    }

    /// Deserialize the format from a MIME type.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ImageFormat, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "image/jpeg" => Ok(ImageFormat::Jpeg),
            "image/png" => Ok(ImageFormat::Png),
            other => Err(serde::de::Error::custom(format!(
                "unsupported image MIME type: {other}"
            ))),
        }
    }
}

#[cfg(test)]
pub mod testing {
    //! Test helpers.

    use super::CoverArt;

    /// A minimal JPEG-looking cover (only the magic bytes matter for sniffing).
    pub fn jpeg_cover() -> CoverArt {
        CoverArt::from_data(vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46]).unwrap()
    }

    /// A minimal PNG-looking cover.
    pub fn png_cover() -> CoverArt {
        CoverArt::from_data(vec![
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3,
        ])
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{jpeg_cover, png_cover};
    use super::*;

    #[test]
    fn test_sniff_jpeg() {
        assert_eq!(
            ImageFormat::sniff(&[0xFF, 0xD8, 0xFF, 0xE1]),
            Some(ImageFormat::Jpeg)
        );
    }

    #[test]
    fn test_sniff_png() {
        assert_eq!(
            ImageFormat::sniff(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]),
            Some(ImageFormat::Png)
        );
    }

    #[test]
    fn test_sniff_unsupported_or_empty() {
        assert_eq!(ImageFormat::sniff(b"GIF89a"), None);
        assert_eq!(ImageFormat::sniff(b"<html>"), None);
        assert_eq!(ImageFormat::sniff(&[0xFF, 0xD8]), None);
        assert_eq!(ImageFormat::sniff(&[]), None);
    }

    #[test]
    fn test_description() {
        assert_eq!(
            jpeg_cover().description(CoverArtResolution::Res500),
            "Front Cover (JPG, 500px, 8 B)"
        );
        assert_eq!(
            png_cover().description(CoverArtResolution::Original),
            "Front Cover (PNG, original, 11 B)"
        );
    }

    #[test]
    fn test_description_with_dimensions() {
        let mut data = png_header(500, 400);
        data.extend_from_slice(&[0; 4]);
        let cover = CoverArt::from_data(data).unwrap();
        assert_eq!(
            cover.description(CoverArtResolution::Res500),
            "Front Cover (PNG, 500x400px, 28 B)"
        );
    }

    /// Build a PNG header with the given dimensions.
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data
    }

    /// Build a JPEG with an APP0 segment followed by a SOF0 segment.
    fn jpeg_header(width: u16, height: u16) -> Vec<u8> {
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        data.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 8]);
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&[1, 1, 0x11, 0]);
        data
    }

    #[test]
    fn test_png_dimensions() {
        assert_eq!(image_dimensions(&png_header(500, 400)), Some((500, 400)));
    }

    #[test]
    fn test_jpeg_dimensions() {
        assert_eq!(image_dimensions(&jpeg_header(1200, 900)), Some((1200, 900)));
    }

    #[test]
    fn test_dimensions_of_truncated_or_unknown_data() {
        assert_eq!(image_dimensions(&[]), None);
        assert_eq!(image_dimensions(b"GIF89a"), None);
        assert_eq!(image_dimensions(&png_header(1, 1)[..16]), None);
        assert_eq!(image_dimensions(&jpeg_header(1, 1)[..12]), None);
        assert_eq!(image_dimensions(&jpeg_cover().data()[..4]), None);
    }

    #[test]
    fn test_cover_info_description() {
        let mut data = jpeg_header(500, 500);
        data.resize(32 * 1024, 0);
        assert_eq!(
            CoverInfo::from_data(&data).description(),
            "Front Cover (JPG, 500x500px, 32 KiB)"
        );
        assert_eq!(
            CoverInfo::from_data(&png_header(250, 250)).description(),
            "Front Cover (PNG, 250x250px, 24 B)"
        );
        assert_eq!(
            CoverInfo::from_data(b"GIF89a").description(),
            "Front Cover (6 B)"
        );
    }

    #[test]
    fn test_mime_types() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.name(), "JPG");
        assert_eq!(ImageFormat::Png.name(), "PNG");
    }

    #[test]
    fn test_from_data_rejects_unsupported() {
        assert!(CoverArt::from_data(b"not an image".to_vec()).is_none());
    }

    #[test]
    fn test_from_data_detects_format() {
        assert_eq!(jpeg_cover().format(), ImageFormat::Jpeg);
        assert_eq!(jpeg_cover().mime_type(), "image/jpeg");
        assert_eq!(png_cover().format(), ImageFormat::Png);
        assert_eq!(png_cover().data()[..4], [0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn test_serde_roundtrip() {
        for cover in [jpeg_cover(), png_cover()] {
            let json = serde_json::to_string(&cover).unwrap();
            let decoded: CoverArt = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, cover);
        }
    }

    #[test]
    fn test_deserialize_rejects_invalid_base64_and_mime() {
        assert!(
            serde_json::from_str::<CoverArt>(r#"{"format":"image/jpeg","data":"!!!"}"#).is_err()
        );
        assert!(serde_json::from_str::<CoverArt>(r#"{"format":"image/gif","data":""}"#).is_err());
    }

    #[test]
    fn test_debug_does_not_dump_data() {
        let debug = format!("{:?}", jpeg_cover());
        assert!(debug.contains("8 bytes"));
    }

    /// Serve a single HTTP response on a local port and return the URL.
    fn serve_once(status: &'static str, body: Vec<u8>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/cover", listener.local_addr().unwrap());
        let _handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf).unwrap();
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        });
        url
    }

    #[test]
    fn test_download_success() {
        let expected = png_cover();
        let url = serve_once("200 OK", expected.data().to_vec());
        let cover = download(&url, "helicon-test").unwrap();
        assert_eq!(cover, expected);
    }

    #[test]
    fn test_download_unsupported_format() {
        let url = serve_once("200 OK", b"GIF89a....".to_vec());
        let err = download(&url, "helicon-test").unwrap_err();
        assert!(matches!(err, crate::Error::UnsupportedCoverArtFormat));
    }

    #[test]
    fn test_download_not_found() {
        let url = serve_once("404 Not Found", Vec::new());
        let err = download(&url, "helicon-test").unwrap_err();
        let crate::Error::CoverArtDownload(inner) = err else {
            panic!("unexpected error: {err:?}");
        };
        assert!(is_not_found(&*inner));
    }

    #[test]
    fn test_is_not_found_other_errors() {
        assert!(!is_not_found(&ureq::Error::StatusCode(500)));
        assert!(is_not_found(&ureq::Error::StatusCode(404)));
        assert!(!is_not_found(&std::io::Error::other("boom")));
    }
}
