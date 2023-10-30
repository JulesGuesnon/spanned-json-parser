use spanned_json_parser::parse;

// #[test]
// fn parse_basics() {
//     let data = r#"
//     {
//         "hello": "wolrd",
//         "vec": [
//             {
//         "num1": 1,
//         "num2": 1.2,
//         "num3": 1.2e12,
//         "num4": -12
//     }
//         ],
//     "is": false,
//     "is_not": true,
//     "empty": null
//     }
//     "#;
//
//     let spanned_value = parse(data).unwrap();
//
//     assert_eq!(spanned_value.start.line, 2);
//     assert_eq!(spanned_value.start.col, 5);
//     assert_eq!(spanned_value.end.line, 15);
//     assert_eq!(spanned_value.end.col, 5);
// }

mod string {
    use spanned_json_parser::{
        parse,
        value::{Number, SpannedValue},
    };

    #[test]
    fn escaped_null_in_key() {
        let data = r#"{"foo\u0000bar": 42}"#;

        let parsed = parse(data).unwrap();

        let object = parsed.value.unwrap_object();

        let key_value: Vec<(&&str, &SpannedValue)> = object.iter().collect();

        let (key, value) = key_value[0];

        let num = value.value.unwrap_number();

        assert_eq!(key, &r#"foo\u0000bar"#);
        assert_eq!(num, &Number::PosInt(42));
    }
}
