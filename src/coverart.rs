// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Cover art support.

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

    /// The raw image data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
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
    fn test_mime_types() {
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
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
