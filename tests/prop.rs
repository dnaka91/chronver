use proptest::proptest;

proptest! {
    #[test]
    fn parse(value: String) {
        value.parse::<chronver::Version>().ok();
    }

    #[test]
    fn parse_date(value: String) {
        value.parse::<chronver::Date>().ok();
    }

    #[test]
    fn date_numbers(value in "\\d+\\.\\d+\\.\\d+") {
        value.parse::<chronver::Date>().ok();
    }

    #[test]
    fn parse_changeset(value: String) {
        value.parse::<chronver::Changeset>().ok();
    }

    #[test]
    fn parse_kind(value: String) {
        value.parse::<chronver::Kind>().ok();
    }
}
