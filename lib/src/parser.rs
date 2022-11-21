#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_now() {
        let input = "jetzt";
        let actual = crate::parse_time_de(input).unwrap();

        let expected = crate::de::Time::Now;

        assert_eq!(actual, expected);
    }
}
