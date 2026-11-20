//! A transcript of nothing but numbers skips the OpenCode rewrite: the
//! prompt's number rules would rather paste as heard than risk a changed
//! figure (#949).

/// Whether a transcript says nothing but numbers: digits or English number
/// words, with punctuation between them.
pub(crate) fn numbers_only(transcript: &str) -> bool {
    const WORDS: &[&str] = &[
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
        "thirty",
        "forty",
        "fifty",
        "sixty",
        "seventy",
        "eighty",
        "ninety",
        "hundred",
        "thousand",
        "million",
        "billion",
    ];
    /// Words that join numbers but are not numbers themselves.
    const JOINERS: &[&str] = &["and", "point", "minus", "negative"];
    let mut any_number = false;
    for token in transcript.split(|c: char| c.is_whitespace() || c == '-') {
        let word = token.trim_matches(|c: char| !c.is_alphanumeric());
        if word.is_empty() {
            continue;
        }
        if word
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
            || WORDS.iter().any(|w| word.eq_ignore_ascii_case(w))
        {
            any_number = true;
        } else if !JOINERS.iter().any(|w| word.eq_ignore_ascii_case(w)) {
            return false;
        }
    }
    any_number
}
