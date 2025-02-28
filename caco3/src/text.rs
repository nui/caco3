/// indents every line in a given string to the specified indent width
///
/// Examples
/// ```rust
/// use caco3::text::indent;
/// assert_eq!(indent(1, ' ', false, "hello"), " hello");
/// ```
pub fn indent(width: usize, ch: char, trim_end: bool, input: &str) -> String {
    let cap = input.len() + (ch.len_utf8() * width * input.lines().count());
    let mut linebuf = String::new();
    if input.is_empty() {
        linebuf.extend(core::iter::repeat(ch).take(width));
        if trim_end {
            linebuf.truncate(linebuf.trim_end().len());
        }
        return linebuf;
    }
    let mut out = input
        .lines()
        .fold(String::with_capacity(cap), |mut acc, line| {
            linebuf.clear();
            linebuf.extend(core::iter::repeat(ch).take(width));
            linebuf.push_str(line);
            if trim_end {
                linebuf.truncate(linebuf.trim_end().len());
            }
            acc.push_str(&linebuf);
            acc.push('\n');
            acc
        });
    // pop the last newline
    out.pop();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(indent(1, ' ', false, ""), " ");
        assert_eq!(indent(1, ' ', true, ""), "");
        assert_eq!(indent(1, ' ', false, " "), "  ");
        assert_eq!(indent(1, ' ', false, "x"), " x");
        assert_eq!(indent(1, '_', false, "x"), "_x");
        assert_eq!(indent(2, ' ', false, "x"), "  x");
        assert_eq!(indent(2, ' ', false, "x "), "  x ");
        assert_eq!(indent(2, ' ', true, "x "), "  x");
        assert_eq!(indent(2, ' ', true, "x\ny"), "  x\n  y");
        assert_eq!(indent(2, ' ', false, "x\n\ny"), "  x\n  \n  y");
        assert_eq!(indent(2, ' ', true, "x\n\ny"), "  x\n\n  y");
    }
}
