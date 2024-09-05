pub mod parser;

#[cfg(test)]
mod tests {
    use crate::parser::*;

    #[test]
    fn check_true_constant() {
        let result = accept_true(JsonFragment {
            stream: "true",
            raw: vec![],
            value: JsonValue::NULL
        });
        assert!(result.is_ok());
        if let Ok(frag) = result {
            assert_eq!(frag.value, JsonValue::TRUE);
        }
    }

    #[test]
    fn check_false_constant() {
        let result = accept_false(JsonFragment {
            stream: "false",
            raw: vec![],
            value: JsonValue::NULL
        });
        assert!(result.is_ok());
        if let Ok(frag) = result {
            assert_eq!(frag.value, JsonValue::FALSE);
        }
    }

    #[test]
    fn check_null_constant() {
        let result = accept_null(JsonFragment {
            stream: "null",
            raw: vec![],
            value: JsonValue::FALSE
        });
        assert!(result.is_ok());
        if let Ok(frag) = result {
            assert_eq!(frag.value, JsonValue::NULL);
        }
    }

    #[test]
    fn check_strings() {
        let cases = [
            "some long value string SLDFJNSDLFN",
            "\\/multi\\b line\\n stri ng000 111\\r with\\t control seqE\\\\UNces",
            "\\u2764\\ubBbB\\u27af\\u2Ef4\\u2cD4\\u2AA4",
            "                                        "
        ];
        for expected in cases {
            let stream = format!("\"{}\"", expected);
            let result = accept_string(JsonFragment {
                stream: &stream[..],
                raw: vec![],
                value: JsonValue::NULL
            });
            assert!(result.is_ok());
            if let Ok(frag) = result {
                assert_eq!(frag.value, JsonValue::STRING(expected.to_string()))
            }
        }
    }

    #[test]
    fn check_numbers() {
        let cases = [
            0f64,
            1f64,
            -1f64,
            -1000f64,
            -5f64,
            17f64,
            3333f64,
            1345.15f64,
            0.1e-05f64
        ];
        for expected in cases {
            let stream = format!("{}", expected);
            let result = accept_number(JsonFragment {
                stream: &stream[..],
                raw: vec![],
                value: JsonValue::NULL
            });
            assert!(result.is_ok());
            if let Ok(frag) = result {
                assert_eq!(frag.value, JsonValue::NUMBER(expected));
            }
        }
    }

    #[test]
    fn check_whitespaces() {
        let cases = [
            "",
            " ",
            "                 ",
            "\r\r\r   \r",
            "\t\t\t  \t",
            "\n\n\n\n\n\r\r\r\t\t\t    "
        ];
        for stream in cases {
            let result = accept_whitespace(JsonFragment {
                stream,
                raw: vec![],
                value: JsonValue::NULL
            });
            assert!(result.is_ok());
            if let Ok(frag) = result {
                assert!(frag.stream.is_empty());
                assert!(frag.raw.is_empty());
                assert_eq!(frag.value, JsonValue::NULL);
            }
        }
    }

    #[test]
    fn check_arrays() {
        let cases = [
            ("[1]", JsonValue::ARRAY(vec![JsonValue::NUMBER(1f64)])),
            ("[ 1, 2, 3 ]", JsonValue::ARRAY(vec![JsonValue::NUMBER(1f64), JsonValue::NUMBER(2f64), JsonValue::NUMBER(3f64)]))
        ];
        for (stream, expected) in cases {
            let result = accept_array(JsonFragment {
                stream,
                raw: vec![],
                value: JsonValue::NULL
            });
            assert!(result.is_ok());
            if let Ok(frag) = result {
                assert!(frag.stream.is_empty());
                assert_eq!(frag.value, expected);
            }
        }
    }
}
