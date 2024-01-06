mod private {
    pub trait Sealed {}

    impl<T> Sealed for T where T: AsRef<str> {}
}

pub trait SqlTrimBoxed: private::Sealed {
    fn sql_trim_boxed(&self) -> Box<str>;
}

fn keep_sql_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    // We do not remove C-style block comments.
    let is_comment_line = || trimmed.starts_with("--");
    let is_blank_line = || trimmed.is_empty();
    let skip = is_comment_line() || is_blank_line();
    let keep = !skip;
    keep.then_some(line)
}

fn sql_trim_boxed(query: &str) -> Box<str> {
    query
        .lines()
        .flat_map(keep_sql_line)
        .collect::<Vec<_>>()
        .join("\n")
        .into_boxed_str()
}

impl<T: AsRef<str>> SqlTrimBoxed for T {
    fn sql_trim_boxed(&self) -> Box<str> {
        sql_trim_boxed(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn sql_trim_boxed() {
        let query = indoc! {r#"
            -- This is comment
            SELECT * FROM users;
        "#};
        let actual = query.sql_trim_boxed();
        let expect = "SELECT * FROM users;";
        assert_eq!(actual.as_ref().trim(), expect);

        let query = indoc! {r#"
            -- PREPARE data_request__update_fsm_state(bigint, fsm_state, timestamptz) AS
            UPDATE data_request
            SET fsm_state     = $2,
                -- Also comment here
                modified_date = $3
            WHERE data_request_id = $1;
            -- Comment after sql
        "#};
        let actual = query.sql_trim_boxed();
        let expect = indoc! {r#"
            UPDATE data_request
            SET fsm_state     = $2,
                modified_date = $3
            WHERE data_request_id = $1;
        "#};
        assert_eq!(actual.as_ref().trim(), expect.trim());
    }
}
