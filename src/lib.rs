extern crate hangeul;
extern crate igo;
extern crate tempfile;
extern crate unicode_normalization;
extern crate wana_kana;
extern crate zip;

use std::fs::File;
use std::io::{self, Cursor};
use std::path::Path;

use igo::Tagger;
use tempfile::TempDir;
use wana_kana::is_katakana::is_katakana;
use wana_kana::to_romaji::to_romaji;
use zip::ZipArchive;

use unicode_normalization::UnicodeNormalization;

pub struct Romanizer {
    // Drop order is top to bottom
    tagger: Tagger,
    // Keep `tempdir` in this struct as the directory is deleted once the struct is dropped
    _tempdir: TempDir,
}

impl Romanizer {
    /// Initialize a new [`Romanizer`]. This takes some time as some dictionary data has to get
    /// extracted to the file sytem and loaded.
    pub fn new() -> Result<Romanizer, io::Error> {
        let tempdir = TempDir::new()?;
        unzip(include_bytes!("../ipadic/ipadic.zip"), tempdir.path())?;

        let tagger = Tagger::new(&tempdir.path()).unwrap();

        Ok(Romanizer {
            tagger,
            _tempdir: tempdir,
        })
    }

    /// # Examples
    ///
    /// ```
    /// let romanizer = romanize::Romanizer::new().unwrap();
    /// assert_eq!(
    ///     romanizer.romanize("U&I ～夕日の綺麗なあの丘で～ U&I"),
    ///     "U&I ~Yūhi no Kirei na ano Oka de~ U&I",
    /// );
    /// ```
    pub fn romanize(&self, input: &str) -> String {
        let mut romanized = hangeul::romanize(input);

        let parts = self.tagger.parse(input);
        let mut insert_space = false;
        // Monotonically increasing index to the last replaced characters
        let mut last_idx = 0;
        for ref part in parts {
            // Part features:
            // 0 Part-of-speech
            // 1 Part-of-speech subdivision class 1
            // 2 Partspeech subdivision class 2
            // 3 Partspeech subdivision class 3
            // 4 Utilization type
            // 5 Utilization form
            // 6 Original form
            // 7 Reading
            // 8 Pronunciation

            let feature = part.feature.split(',').collect::<Vec<_>>();

            // Don't change punctuation; also don't update idx as other occurences will be found at
            // places before the place of this part
            if feature[0] == "記号" {
                insert_space = false;
                continue;
            }

            let idx = romanized.find(part.surface).unwrap();

            let katakana = feature.get(8).or(if is_katakana(part.surface) {
                Some(&part.surface)
            } else {
                None
            });

            if let Some(katakana) = katakana {
                let mut replacement = to_romaji(katakana);

                // Capitalize nouns
                if feature[0] == "名詞" {
                    replacement = uppercase_first_character(&replacement);
                }

                replacement = replacement.replace('-', "\u{0304}");

                if insert_space {
                    replacement.insert(0, ' ');
                }

                romanized = romanized.replacen(part.surface, &replacement, 1);
                insert_space = true;
            } else {
                // Only insert space if another word comes afterwards
                if insert_space && part
                    .surface
                    .chars()
                    .next()
                    .map(|c| c.is_alphanumeric())
                    .unwrap_or(false)
                {
                    let idx = romanized[last_idx..].find(part.surface).unwrap();
                    romanized.insert(idx + last_idx, ' ');
                }
                insert_space = false;
            }

            last_idx = idx;
        }

        romanized
            .nfkc() // Normalize unicode
            .to_string()
    }
}

// From https://stackoverflow.com/a/38406885
fn uppercase_first_character(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

fn unzip(zip: &[u8], output_directory: &Path) -> Result<(), io::Error> {
    // Use unwraps as we control the zip
    let mut archive = ZipArchive::new(Cursor::new(zip)).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let file_path = file.sanitized_name();

        // Only extract files (not directories) directly into output_directory
        // (suffices for our use case)
        if !(file.name()).ends_with('/') {
            let mut outfile = File::create(&output_directory.join(file_path.file_name().unwrap()))?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romanize() {
        let romanizer = Romanizer::new().unwrap();
        assert_eq!(romanizer.romanize("太陽のKiss"), "Taiyō no Kiss");
        assert_eq!(
            romanizer.romanize("エブリデイワールド"),
            "Eburideiwārudo"
        );
        assert_eq!(
            romanizer.romanize("U&I ～夕日の綺麗なあの丘で～ U&I"),
            "U&I ~Yūhi no Kirei na ano Oka de~ U&I",
        );
        assert_eq!(
            romanizer.romanize("ふでペン ～ボールペン～ [GAME Mix]"),
            "fu de Pen ~Bōrupen~ [GAME Mix]",
        );
        assert_eq!(
            romanizer.romanize("空の境界 「殺人考察（後）」Original Soundtrack"),
            "Sora no Kyōkai 「Satsujin Kōsatsu(Go)」Original Soundtrack",
        );
    }
}
