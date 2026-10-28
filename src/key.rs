// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Musical keys and their notations.

use serde::{Deserialize, Serialize};

/// Notation formats for writing musical keys to tags.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyNotation {
    /// Traditional musical notation: "C Major", "A Minor", etc.
    #[default]
    Traditional,
    /// Camelot notation: "8B", "8A", etc. (minor = A, major = B).
    Camelot,
    /// Open Key notation: "1d", "1m", etc. (minor = m, major = d).
    OpenKey,
    /// ID3v2 "Initial key" notation: "C", "Am", "Bb", etc.
    ///
    /// This is mandated by the ID3v2 specification for the `TKEY` frame and is enforced by the ID3
    /// tag implementation, so there is usually no point in selecting it in the configuration.
    #[serde(rename = "id3v2")]
    ID3v2,
}

/// The tonic (root note) of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tonic {
    /// C.
    C,
    /// D♭.
    DFlat,
    /// D.
    D,
    /// E♭.
    EFlat,
    /// E.
    E,
    /// F.
    F,
    /// G♭.
    GFlat,
    /// G.
    G,
    /// A♭.
    AFlat,
    /// A.
    A,
    /// B♭.
    BFlat,
    /// B.
    B,
}

impl Tonic {
    /// Pitch class (semitones above C).
    const fn pitch_class(self) -> u8 {
        self as u8
    }

    /// Position on the circle of fifths (C = 0, G = 1, D = 2, ...).
    const fn fifths(self) -> u8 {
        (self.pitch_class() * 7) % 12
    }

    /// Name using ASCII accidentals.
    const fn ascii_name(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::DFlat => "Db",
            Self::D => "D",
            Self::EFlat => "Eb",
            Self::E => "E",
            Self::F => "F",
            Self::GFlat => "Gb",
            Self::G => "G",
            Self::AFlat => "Ab",
            Self::A => "A",
            Self::BFlat => "Bb",
            Self::B => "B",
        }
    }

    /// Name using Unicode accidentals.
    const fn unicode_name(self) -> &'static str {
        match self {
            Self::DFlat => "D♭",
            Self::EFlat => "E♭",
            Self::GFlat => "G♭",
            Self::AFlat => "A♭",
            Self::BFlat => "B♭",
            _ => self.ascii_name(),
        }
    }
}

/// A musical key (independent of any analyzer or notation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicalKey {
    /// Major key with the given tonic.
    Major(Tonic),
    /// Minor key with the given tonic.
    Minor(Tonic),
    /// No key could be detected (e.g. silence).
    Silence,
}

impl MusicalKey {
    /// Number on the Camelot wheel (1-12), where major and relative minor share a number.
    fn camelot_number(tonic: Tonic, minor: bool) -> u8 {
        let major_tonic_fifths = if minor {
            // The relative major is three semitones above the minor tonic.
            (((tonic.pitch_class() + 3) % 12) * 7) % 12
        } else {
            tonic.fifths()
        };
        (major_tonic_fifths + 7) % 12 + 1
    }

    /// Format the key in the given notation.
    ///
    /// Returns `None` if the key cannot be expressed in this notation (e.g. silence).
    #[must_use]
    pub fn format(self, notation: KeyNotation) -> Option<String> {
        let (tonic, minor) = match self {
            Self::Major(tonic) => (tonic, false),
            Self::Minor(tonic) => (tonic, true),
            Self::Silence => {
                return matches!(notation, KeyNotation::ID3v2).then(|| "o".to_string());
            }
        };
        let formatted = match notation {
            KeyNotation::Traditional => format!(
                "{} {}",
                tonic.unicode_name(),
                if minor { "Minor" } else { "Major" }
            ),
            KeyNotation::Camelot => format!(
                "{}{}",
                Self::camelot_number(tonic, minor),
                if minor { 'A' } else { 'B' }
            ),
            KeyNotation::OpenKey => {
                // Open Key is the Camelot wheel rotated so that C major is 1d.
                let number = (Self::camelot_number(tonic, minor) + 4) % 12 + 1;
                format!("{number}{}", if minor { 'm' } else { 'd' })
            }
            KeyNotation::ID3v2 => {
                format!("{}{}", tonic.ascii_name(), if minor { "m" } else { "" })
            }
        };
        Some(formatted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        let c_major = MusicalKey::Major(Tonic::C);
        let a_minor = MusicalKey::Minor(Tonic::A);
        let b_flat_minor = MusicalKey::Minor(Tonic::BFlat);

        assert_eq!(c_major.format(KeyNotation::Camelot).unwrap(), "8B");
        assert_eq!(a_minor.format(KeyNotation::Camelot).unwrap(), "8A");
        assert_eq!(
            MusicalKey::Major(Tonic::B)
                .format(KeyNotation::Camelot)
                .unwrap(),
            "1B"
        );
        assert_eq!(
            MusicalKey::Minor(Tonic::AFlat)
                .format(KeyNotation::Camelot)
                .unwrap(),
            "1A"
        );

        assert_eq!(c_major.format(KeyNotation::OpenKey).unwrap(), "1d");
        assert_eq!(a_minor.format(KeyNotation::OpenKey).unwrap(), "1m");
        assert_eq!(
            MusicalKey::Major(Tonic::G)
                .format(KeyNotation::OpenKey)
                .unwrap(),
            "2d"
        );
        assert_eq!(
            MusicalKey::Major(Tonic::F)
                .format(KeyNotation::OpenKey)
                .unwrap(),
            "12d"
        );

        assert_eq!(
            b_flat_minor.format(KeyNotation::Traditional).unwrap(),
            "B♭ Minor"
        );
        assert_eq!(b_flat_minor.format(KeyNotation::ID3v2).unwrap(), "Bbm");
        assert_eq!(c_major.format(KeyNotation::ID3v2).unwrap(), "C");
    }

    #[test]
    fn test_format_silence() {
        assert_eq!(
            MusicalKey::Silence.format(KeyNotation::ID3v2).as_deref(),
            Some("o")
        );
        assert!(MusicalKey::Silence.format(KeyNotation::Camelot).is_none());
    }
}
