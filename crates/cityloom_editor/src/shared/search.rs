//! Text folded for search: lower case, no accents, plain apostrophe and spaces.

/// The text as a search compares it: lower case, French accents removed, `’` as `'`, no-break spaces as spaces.
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            '\u{2019}' => '\'',
            '\u{a0}' | '\u{202f}' => ' ',
            _ => unaccent(c).unwrap_or(c),
        })
        .collect()
}

/// The base letter of a lower-case French accented letter; `œ` has none and stays.
pub fn unaccent(c: char) -> Option<char> {
    Some(match c {
        'à' | 'â' | 'ä' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'î' | 'ï' => 'i',
        'ô' | 'ö' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ÿ' => 'y',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_drops_case_accents_and_typographic_characters() {
        assert_eq!(fold("Arrêt sur rue"), "arret sur rue");
        assert_eq!(fold("Café terrace"), "cafe terrace");
        assert_eq!(fold("Avancée\u{a0}de trottoir"), "avancee de trottoir");
        assert_eq!(fold("l\u{2019}arbre"), "l'arbre");
        assert_eq!(fold("ÉLÉMENT"), "element");
    }

    #[test]
    fn unaccent_knows_the_french_letters_only() {
        assert_eq!(unaccent('é'), Some('e'));
        assert_eq!(unaccent('ç'), Some('c'));
        assert_eq!(unaccent('a'), None);
    }
}
