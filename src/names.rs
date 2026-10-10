//! Aslan name generation, built from the Trokh sound inventory.
//! Names are assembled from Trokh phonemes only, following the patterns of
//! the attested names (Hlyueawi, Ahroay'i, Wahtoi, Iwahfea): a personal name
//! followed by a clan name, where the clan name ends in an `-i` suffix.

use crate::model::Rng;

/// Trokh vowels and diphthongs.
const VOWELS: &[&str] = &["a", "ae", "ao", "e", "ea", "i", "o", "u", "ya", "ye", "iy", "yo", "yu"];
/// Vowels that read well as the last sound of a personal name.
const FINAL_VOWELS: &[&str] = &["a", "ae", "ao", "ea", "i", "o", "u", "ya", "yu"];
/// Trokh consonants and clusters (the glottal stop is handled separately).
const ONSETS: &[&str] = &[
    "f", "ft", "h", "hf", "hk", "hl", "hr", "ht", "hw", "k", "kh", "kht", "l", "lr", "r", "rl",
    "s", "st", "t", "tl", "tr", "w",
];
/// Consonants that close a syllable; the harsher clusters stay word-initial.
const CODAS: &[&str] = &["f", "h", "k", "kh", "l", "r", "s", "t", "w"];

/// Clan-name suffixes: attested clans end in -i, -wi, -oi and -ei.
const CLAN_ENDINGS: &[&str] = &["i", "i", "wi", "oi", "ei"];

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Builds one pronounceable Trokh word with `syllables` syllables.
fn trokh_word(r: &mut Rng, syllables: usize, ending: Option<&str>) -> String {
    let mut w = String::new();
    for i in 0..syllables {
        let last = i + 1 == syllables;
        if i == 0 {
            // Words may open on a vowel (Ahroay'i, Iwahfea) but usually take an onset.
            if r.below(5) != 0 {
                w += *r.pick(ONSETS);
            }
        } else if r.below(6) == 0 {
            // Glottal stop between vowels, as in Ahroay'i and Aroaye'i.
            w.push('\'');
        } else {
            w += *r.pick(ONSETS);
        }
        w += *if last && ending.is_none() { r.pick(FINAL_VOWELS) } else { r.pick(VOWELS) };
    }
    match ending {
        Some(e) => w + e,
        None => {
            if r.below(3) == 0 {
                w += *r.pick(CODAS);
            }
            w
        }
    }
}

/// A personal name, e.g. "Ftahea" or "Khtyaolu".
pub fn aslan_given(r: &mut Rng) -> String {
    let syllables = 2 + r.below(2);
    capitalize(&trokh_word(r, syllables, None))
}

/// A clan name, e.g. "Wahtoi" or "Hlyueawi".
pub fn aslan_clan(r: &mut Rng) -> String {
    let syllables = 2 + r.below(2);
    let ending = *r.pick(CLAN_ENDINGS);
    capitalize(&trokh_word(r, syllables, Some(ending)))
}

/// Full Aslan name: personal name then clan name.
pub fn aslan_name(r: &mut Rng) -> String {
    let given = aslan_given(r);
    let clan = aslan_clan(r);
    format!("{given} {clan}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aslan_names_look_right() {
        let mut r = Rng::new(99);
        for _ in 0..2000 {
            let n = aslan_name(&mut r);
            let (given, clan) = n.split_once(' ').expect("given and clan");
            assert!(given.chars().next().unwrap().is_uppercase());
            assert!(clan.chars().next().unwrap().is_uppercase());
            assert!(clan.ends_with('i'));
            assert!(!n.starts_with('\''));
            assert!(n.chars().all(|c| c.is_alphabetic() || c == '\'' || c == ' '));
            assert!(n.len() >= 5 && n.len() < 40, "{n}");
        }
    }

    #[test]
    fn print_aslan_samples() {
        let mut r = Rng::new(5);
        for _ in 0..12 {
            println!("{}", aslan_name(&mut r));
        }
    }
}
