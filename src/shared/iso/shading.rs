//! The filters that darken the faces turned from the eye in a 2.5D drawing.

/// The `<filter>` markup for each `(id, slope)`: every colour channel is multiplied by the slope (in sRGB, so
/// it darkens as CSS `brightness()` does). SVG filters, not CSS `filter: brightness()`, which Safari may not
/// apply to SVG shapes. The caller puts them in its own `<defs>`.
pub fn shade_filters(shades: &[(&str, &str)]) -> String {
    shades
        .iter()
        .map(|(id, k)| {
            format!(
                "<filter id=\"{id}\" color-interpolation-filters=\"sRGB\"><feComponentTransfer><feFuncR type=\"linear\" slope=\"{k}\"/><feFuncG type=\"linear\" slope=\"{k}\"/><feFuncB type=\"linear\" slope=\"{k}\"/></feComponentTransfer></filter>"
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filter_is_written_for_each_id_with_its_slope_on_all_three_channels() {
        let one = shade_filters(&[("f", "0.5")]);
        assert_eq!(
            one,
            "<filter id=\"f\" color-interpolation-filters=\"sRGB\"><feComponentTransfer><feFuncR type=\"linear\" slope=\"0.5\"/><feFuncG type=\"linear\" slope=\"0.5\"/><feFuncB type=\"linear\" slope=\"0.5\"/></feComponentTransfer></filter>"
        );
        let two = shade_filters(&[("a", "0.9"), ("b", "0.78")]);
        assert!(two.starts_with("<filter id=\"a\"") && two.contains("</filter><filter id=\"b\""));
        assert_eq!(two.matches("slope=\"0.78\"").count(), 3);
        assert_eq!(shade_filters(&[]), "");
    }
}
