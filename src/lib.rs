pub mod parser;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::parser::*;

    #[test]
    fn check_true_constant() {
        prepare_environment("true".to_string());
        let result = accept_true(JsonValue::NULL);
        assert!(result.is_ok());
        if let Ok(jval) = result {
            assert_eq!(jval, JsonValue::TRUE);
        }
    }

    #[test]
    fn check_false_constant() {
        prepare_environment("false".to_string());
        let result = accept_false(JsonValue::NULL);
        assert!(result.is_ok());
        if let Ok(jval) = result {
            assert_eq!(jval, JsonValue::FALSE);
        }
    }

    #[test]
    fn check_null_constant() {
        prepare_environment("null".to_string());
        let result = accept_null(JsonValue::NULL);
        assert!(result.is_ok());
        if let Ok(jval) = result {
            assert_eq!(jval, JsonValue::NULL);
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
            prepare_environment(stream);
            let result = accept_string(JsonValue::NULL);
            assert!(result.is_ok());
            if let Ok(jval) = result {
                assert_eq!(jval, JsonValue::STRING(expected.to_string()))
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
            prepare_environment(stream);
            let result = accept_number(JsonValue::NULL);
            assert!(result.is_ok());
            if let Ok(jval) = result {
                assert_eq!(jval, JsonValue::NUMBER(expected));
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
            prepare_environment(stream.to_string());
            let result = accept_whitespace(JsonValue::NULL);
            assert!(result.is_ok());
            if let Ok(jval) = result {
                assert_eq!(jval, JsonValue::NULL);
            }
        }
    }

    #[test]
    fn check_arrays() {
        let cases = [
            ("[1]", JsonValue::ARRAY(vec![JsonValue::NUMBER(1f64)])),
            ("[ 1, 2, 3 ]", JsonValue::ARRAY(vec![
                JsonValue::NUMBER(1f64),
                JsonValue::NUMBER(2f64),
                JsonValue::NUMBER(3f64)]))
        ];
        for (stream, expected) in cases {
            prepare_environment(stream.to_string());
            let result = accept_array(JsonValue::NULL);
            assert!(result.is_ok());
            if let Ok(jval) = result {
                assert_eq!(jval, expected);
            }
        }
    }

    #[test]
    fn check_object() {
        let stream = "{
    \"3.18\": {
        \"3.18.1\": {
            \"CVE-2014-8559\": {
                \"cmt_msg\": \"crypto: prefix module autoloading with \\\"crypto-\\\"\", 
                \"cmt_id\": \"679829c2e50332832c2e85b12ec851a423ad9892\"
            }
        }
    }
}".to_string();
        prepare_environment(stream);
        let result = accept_object(JsonValue::NULL);
        assert!(result.is_ok());
        if let Ok(jval) = result {
            assert_eq!(jval, JsonValue::OBJECT(HashMap::from([
                ("3.18".to_string(), Box::new(JsonValue::OBJECT(HashMap::from([
                    ("3.18.1".to_string(), Box::new(JsonValue::OBJECT(HashMap::from([
                        ("CVE-2014-8559".to_string(), Box::new(JsonValue::OBJECT(HashMap::from([
                            ("cmt_msg".to_string(), Box::new(JsonValue::STRING("crypto: prefix module autoloading with \\\"crypto-\\\"".to_string()))),
                            ("cmt_id".to_string(), Box::new(JsonValue::STRING("679829c2e50332832c2e85b12ec851a423ad9892".to_string())))
                        ]))))
                    ]))))
                ]))))
            ])));
        }
    }
}
