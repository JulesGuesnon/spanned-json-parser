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

// mod string {
//     use spanned_json_parser::{
//         parse,
//         value::{Number, SpannedValue},
//     };
//
//     #[test]
//     fn emoji_in_key() {
//         let data = r#"{"foo🤔bar": 42}"#;
//
//         let parsed = parse(data).unwrap();
//
//         let object = parsed.value.unwrap_object();
//
//         let key_value: Vec<(&String, &SpannedValue)> = object.iter().collect();
//
//         let (key, value) = key_value[0];
//
//         let num = value.value.unwrap_number();
//
//         assert_eq!(key, &r#"foo🤔bar"#);
//         assert_eq!(num, &Number::PosInt(42));
//     }
//
//     #[test]
//     fn escaped_null_in_key() {
//         let data = r#"{"foo\u0000bar": 42}"#;
//
//         let parsed = parse(data).unwrap();
//
//         let object = parsed.value.unwrap_object();
//
//         let key_value: Vec<(&String, &SpannedValue)> = object.iter().collect();
//
//         let (key, value) = key_value[0];
//
//         let num = value.value.unwrap_number();
//
//         assert_eq!(key, &"foo\u{0000}bar");
//         assert_eq!(num, &Number::PosInt(42));
//     }
// }
//
// mod number {
//     use spanned_json_parser::parse;
//
//     #[test]
//     fn parse_exp() {
//         let data = "[123e65]";
//
//         let parsed = parse(data);
//
//         parsed.unwrap();
//         // assert!(parsed.is_ok());
//     }
//
//     #[test]
//     fn parse_too_big_pos_int() {
//         let data = "[100000000000000000000]";
//
//         let parsed = parse(data);
//
//         assert!(parsed.is_err());
//     }
//
//     #[test]
//     fn parse_padded_number() {
//         let data = "[01]";
//
//         let parsed = parse(data);
//
//         assert!(parsed.is_err());
//
//         let data = "[00000001]";
//
//         let parsed = parse(data);
//
//         assert!(parsed.is_err());
//
//         let data = "[0]";
//
//         let parsed = parse(data);
//
//         assert!(parsed.is_ok());
//     }
// }
//
// mod array {
//     use spanned_json_parser::parse;
//
//     #[test]
//     fn extra_bracket() {
//         let data = "[\"x\"]]";
//
//         let parsed = parse(data);
//
//         assert!(parsed.is_err());
//     }
//
//     #[test]
//     fn invalid_utf8() {
//         let data = r#"[�]"#;
//
//         let parsed = parse(data);
//
//         assert!(parsed.is_err());
//     }
// }
