use spanned_json_parser::{parse, value::Number};

#[test]
fn parse_basic() {
    let data = r#"
    {
        "hello": "world",
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

    let root = spanned_value.value.unwrap_object();
    let world = root.get("hello").unwrap();

    assert_eq!(world.start.line, 3);
    assert_eq!(world.start.col, 18);
    assert_eq!(world.end.line, 3);
    assert_eq!(world.end.col, 24);
    assert_eq!(world.value.unwrap_string(), "world");
    let vec = root.get("vec").unwrap().value.unwrap_array();

    let num_obj = vec.get(0).unwrap().value.unwrap_object();

    assert_eq!(
        num_obj.get("num1").unwrap().value.unwrap_number(),
        &Number::PosInt(1)
    );
    assert_eq!(
        num_obj.get("num2").unwrap().value.unwrap_number(),
        &Number::Float(1.2)
    );

    assert_eq!(
        num_obj.get("num3").unwrap().value.unwrap_number(),
        &Number::Float(1.2e12)
    );

    assert_eq!(
        num_obj.get("num4").unwrap().value.unwrap_number(),
        &Number::NegInt(-12)
    );
}

mod error {
    use spanned_json_parser::{error::Kind, parse};

    #[test]
    fn invalid_root_json_value() {
        let json = r#"'sussy string'"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 1);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 6);
                assert_eq!(e.kind, Kind::InvalidValue("'sussy".into()));
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn invalid_nested_json_value() {
        let json = r#"{"hello": 123aze}"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 11);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 16);
                assert_eq!(e.kind, Kind::InvalidValue("123aze".into()));
            }
            Ok(_) => panic!("Not supposed to happen"),
        }

        let json = r#"[123aze]"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 2);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 7);
                assert_eq!(e.kind, Kind::InvalidValue("123aze".into()));
            }
            Ok(_) => panic!("Not supposed to happen"),
        }

        let json = r#"{"hello": nul }"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 11);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 13);
                assert_eq!(e.kind, Kind::InvalidValue("nul".into()));
            }
            Ok(_) => panic!("Not supposed to happen"),
        }

        let json = r#"{"hello": vrai
        }"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 11);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 14);
                assert_eq!(e.kind, Kind::InvalidValue("vrai".into()));
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn invalid_key() {
        let json = r#"{   12: "world"}"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 5);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 6);
                assert_eq!(e.kind, Kind::InvalidKey("12".into()));
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn missing_key() {
        let json = r#"{   : "world"}"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 2);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 4);
                assert_eq!(e.kind, Kind::InvalidKey("".into()));
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn missing_colon() {
        let json = r#"{"hello" "world"}"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 9);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 9);
                assert_eq!(e.kind, Kind::MissingColon);
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn nested_failure() {
        let json = r#"["hello"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 2);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 7);
                assert_eq!(e.kind, Kind::MissingQuote);
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn missing_array_bracket() {
        let json = r#"["hello""#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 1);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 8);
                assert_eq!(e.kind, Kind::MissingArrayBracket);
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn missing_object_bracket() {
        let json = r#"{"hello": 1"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 1);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 11);
                assert_eq!(e.kind, Kind::MissingObjectBracket);
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn nested_object_failure() {
        let json = r#"{"hello" 1"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 9);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 9);
                assert_eq!(e.kind, Kind::MissingColon);
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn missing_quote() {
        let json = r#"{"hello": "world}"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 11);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 17);
                assert_eq!(e.kind, Kind::MissingQuote)
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }
}

mod string {
    use spanned_json_parser::{
        parse,
        value::{Number, SpannedValue},
    };

    #[test]
    fn emoji_in_key() {
        let data = r#"{"foo🤔bar": 42}"#;

        let parsed = parse(data).unwrap();

        let object = parsed.value.unwrap_object();

        let key_value: Vec<(&String, &SpannedValue)> = object.iter().collect();

        let (key, value) = key_value[0];

        let num = value.value.unwrap_number();

        assert_eq!(key, &r#"foo🤔bar"#);
        assert_eq!(num, &Number::PosInt(42));
    }

    #[test]
    fn escaped_null_in_key() {
        let data = r#"{"foo\u0000bar": 42}"#;

        let parsed = parse(data).unwrap();

        let object = parsed.value.unwrap_object();

        let key_value: Vec<(&String, &SpannedValue)> = object.iter().collect();

        let (key, value) = key_value[0];

        let num = value.value.unwrap_number();

        assert_eq!(key, &"foo\u{0000}bar");
        assert_eq!(num, &Number::PosInt(42));
    }
}

mod number {
    use spanned_json_parser::{parse, value::Number};

    #[test]
    fn parse_exp() {
        let data = "[123e65]";

        let parsed = parse(data);

        parsed.unwrap();
        // assert!(parsed.is_ok());
    }

    #[test]
    fn parse_too_big_pos_int() {
        let data = "[100000000000000000000]";

        let parsed = parse(data);

        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        let vec = parsed.value.unwrap_array().get(0).unwrap();
        let num = vec.value.unwrap_number();

        assert_eq!(num, &Number::Float(1e20));
    }

    #[test]
    fn parse_padded_number() {
        let data = "[01]";

        let parsed = parse(data);

        assert!(parsed.is_err());

        let data = "[00000001]";

        let parsed = parse(data);

        assert!(parsed.is_err());

        let data = "[0]";

        let parsed = parse(data);

        assert!(parsed.is_ok());
    }
}

mod array {
    use spanned_json_parser::{error::Kind, parse};

    #[test]
    fn extra_bracket() {
        let data = "[\"x\"]]";

        let parsed = parse(data);

        assert!(parsed.is_err());
    }

    #[test]
    fn invalid_utf8() {
        let data = r#"[�]"#;

        let parsed = parse(data);

        assert!(parsed.is_err());
    }

    #[test]
    fn empty() {
        let json = "[]";

        let parsed = parse(json);

        assert!(parsed.is_ok());

        let parsed = parsed.unwrap();

        assert_eq!(parsed.value.unwrap_array().len(), 0);
    }

    #[test]
    fn nested() {
        let json = "[  [[[[ ]  ]  ]]]";

        let parsed = parse(json);

        assert!(parsed.is_ok());

        let parsed = parsed.unwrap();

        let parsed = parsed.value.unwrap_array().get(0).unwrap();
        let parsed = parsed.value.unwrap_array().get(0).unwrap();
        let parsed = parsed.value.unwrap_array().get(0).unwrap();
        let parsed = parsed.value.unwrap_array().get(0).unwrap();

        assert_eq!(parsed.value.unwrap_array().len(), 0);
    }

    #[test]
    fn nested_missing_bracket() {
        let json = "[  [[[[   ]  ]]]";

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 1);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 16);
                assert_eq!(e.kind, Kind::MissingArrayBracket)
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn missing_comma() {
        let json = "[1 2]";

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 1);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 3);
                assert_eq!(e.kind, Kind::MissingComma)
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }
}

mod object {
    use spanned_json_parser::{error::Kind, parse};

    #[test]
    fn empty() {
        let json = "{}";

        let parsed = parse(json);

        assert!(parsed.is_ok());

        let parsed = parsed.unwrap();

        assert_eq!(parsed.value.unwrap_object().len(), 0);
    }

    #[test]
    fn nested() {
        let json = r#"{"h": {"e":    {"l": {"l": {"o": {    }  }  }}  }}"#;

        let parsed = parse(json);

        assert!(parsed.is_ok());

        let parsed = parsed.unwrap();
        let parsed = parsed.value.unwrap_object();
        let parsed = parsed.get("h").unwrap().value.unwrap_object();
        let parsed = parsed.get("e").unwrap().value.unwrap_object();
        let parsed = parsed.get("l").unwrap().value.unwrap_object();
        let parsed = parsed.get("l").unwrap().value.unwrap_object();
        let parsed = parsed.get("o").unwrap().value.unwrap_object();

        assert_eq!(parsed.len(), 0);
    }

    #[test]
    fn nested_missing_bracket() {
        let json = r#"{"h": {"e":    {"l": {"l": {"o": {      "#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 34);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 40);
                assert_eq!(e.kind, Kind::MissingObjectBracket)
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }

    #[test]
    fn missing_comma() {
        let json = r#"{"hello": 2 "world": 12}"#;

        let parsed = parse(json);

        assert!(parsed.is_err());

        match parsed {
            Err(e) => {
                assert_eq!(e.start.line, 1);
                assert_eq!(e.start.col, 1);
                assert_eq!(e.end.line, 1);
                assert_eq!(e.end.col, 12);
                assert_eq!(e.kind, Kind::MissingComma)
            }
            Ok(_) => panic!("Not supposed to happen"),
        }
    }
}
