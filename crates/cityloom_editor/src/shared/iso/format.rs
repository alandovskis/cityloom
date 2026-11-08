//! The two small formatters the 2.5D drawings and the plans all need.

/// `s` as text in SVG markup: the characters that would open or close markup are written as entities.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A number to a tenth, with no trailing zero.
pub fn f1(n: f64) -> String {
    let r = (n * 10.0).round() / 10.0;
    if r == 0.0 { "0".to_string() } else { format!("{r}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_writes_the_four_characters_that_would_start_or_end_markup_as_entities() {
        assert_eq!(esc("a & <b> \"c\""), "a &amp; &lt;b&gt; &quot;c&quot;");
        assert_eq!(esc("plain"), "plain");
        assert_eq!(esc(""), "");
    }

    #[test]
    fn f1_rounds_to_a_tenth_and_leaves_no_trailing_zero_and_no_negative_zero() {
        assert_eq!(f1(12.34), "12.3");
        assert_eq!(f1(12.0), "12");
        assert_eq!(f1(0.25), "0.3");
        assert_eq!(f1(-0.04), "0");
        assert_eq!(f1(-3.26), "-3.3");
    }
}
