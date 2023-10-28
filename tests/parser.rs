use spanned_json_parser::parse;

#[test]
fn parse_basics() {
    let data = r#"
    {
        "hello": "wolrd",
        "vec": [
            {
        "num1": 1,
        "num2": 1.2,
        "num3": 1.2e12,
        "num4": -12
    }
        ],
    "is": false,
    "is_not": true,
    "empty": null
    }
    "#;

    let spanned_value = parse(data).unwrap();

    assert_eq!(spanned_value.start.line, 2);
    assert_eq!(spanned_value.start.col, 5);
    assert_eq!(spanned_value.end.line, 15);
    assert_eq!(spanned_value.end.col, 5);
}
