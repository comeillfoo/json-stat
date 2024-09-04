mod parser {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};


    struct ParseError {
        row: usize,
        col: usize,
        msg: String
    }


    enum JsonValue {
        STRING(String),
        NUMBER(f64),
        OBJECT(HashMap<String, JsonValue>),
        ARRAY(Vec<JsonValue>),
        TRUE,
        FALSE,
        NULL
    }

    fn accept(stream: &str, expected: char) -> Result<&str, &str> {
        if ! stream.is_empty() && stream[0] == expected {
            return Ok(&stream[1..]);
        }
        Err(stream)
    }

    fn just_accept(stream: &str) -> Result<&str, &str> {
        Ok(stream)
    }

    fn accept_cb(expected: char) -> impl FnOnce(&str) -> Result<&str, &str> {
        |stream: &str| accept(stream, expected)
    }

    fn accept_ignoring_case(stream: &str, expected: char) -> Result<&str, &str> {
        accept(stream, expected.to_ascii_lowercase())
            .or(accept(stream, expected.to_ascii_uppercase()))
    }

    fn accept_ignoring_case_cb(expected: char) -> impl FnOnce(&str) -> Result<&str, &str> {
        |stream: &str| accept_ignoring_case(stream, expected)
    }

    fn accept_whitespace(stream: &str) -> Result<&str, &str> {
        accept(stream, ' ')
            .or_else(accept_cb('\n'))
            .or_else(accept_cb('\r'))
            .or_else(accept_cb('\t'))
            .or(Ok(stream))
    }

    fn accept_true(stream: &str) -> Result<&str, &str> {
        accept(stream, 't')
            .and_then(accept_cb('r'))
            .and_then(accept_cb('u'))
            .and_then(accept_cb('e'))
    }

    fn accept_false(stream: &str) -> Result<&str, &str> {
        accept(stream, 'f')
            .and_then(accept_cb('a'))
            .and_then(accept_cb('l'))
            .and_then(accept_cb('s'))
            .and_then(accept_cb('e'))
    }

    fn accept_null(stream: &str) -> Result<&str, &str> {
        accept(stream, 'n')
        .and_then(accept_cb('u'))
        .and_then(accept_cb('l'))
        .and_then(accept_cb('l'))
    }

    fn accept_nonzero(stream: &str) -> Result<&str, &str> {
        accept(stream, '1')
            .or_else(accept_cb('2'))
            .or_else(accept_cb('3'))
            .or_else(accept_cb('4'))
            .or_else(accept_cb('5'))
            .or_else(accept_cb('6'))
            .or_else(accept_cb('7'))
            .or_else(accept_cb('8'))
            .or_else(accept_cb('9'))
    }

    fn accept_digit(stream: &str) -> Result<&str, &str> {
        accept_nonzero(stream).or_else(accept_cb('0'))
    }

    fn accept_digits(stream: &str) -> Result<&str, &str> {
        let digit = accept_digit(stream);
        if digit.is_err() {
            return digit;
        }
        digit.and_then(accept_digits)
            .or_else(|advanced_stream| Ok(advanced_stream))
    }

    fn accept_exponent(stream: &str) -> Result<&str, &str> {
        accept_ignoring_case(stream, 'e')
            .and_then(|advanced_stream| accept(advanced_stream, '+')
                .or_else(accept_cb('-'))
                .or(Ok(advanced_stream)))
            .and_then(accept_digits)
    }

    fn accept_fraction(stream: &str) -> Result<&str, &str> {
        accept(stream, '.')
            .and_then(accept_digits)
    }

    fn accept_integer(stream: &str) -> Result<&str, &str> {
        accept(stream, '-')
            .or(Ok(stream))
            .and_then(accept_cb('0'))
            .or_else(|advanced_stream| accept_nonzero(advanced_stream)
                .and_then(accept_digits)
                .or_else(just_accept))
    }

    fn accept_number(stream: &str) -> Result<&str, &str> {
        let integer = accept_integer(stream);
        if integer.is_err() {
            return integer;
        }
        integer.and_then(accept_fraction)
            .or_else(just_accept)
            .and_then(accept_exponent)
            .or_else(just_accept)
    }

    fn accept_values(stream: &str) -> Result<&str, &str> {
        let first_value = accept_value(stream);
        if first_value.is_err() {
            return first_value;
        }
        first_value
            .and_then(accept_cb(','))
            .and_then(accept_values)
            .or_else(|advanced_stream| Ok(advanced_stream))
    }

    fn accept_array(stream: &str) -> Result<&str, &str> {
        accept(stream, '[')
            .and_then(|advanced_stream| accept_whitespace(advanced_stream)
                .or(accept_values(advanced_stream)))
            .and_then(accept_cb(']'))
    }

    fn accept_key_value(stream: &str) -> Result<&str, &str> {
        accept_whitespace(stream)
            .and_then(accept_string)
            .and_then(accept_whitespace)
            .and_then(accept_cb(':'))
            .and_then(accept_value)
    }

    fn accept_key_values(stream: &str) -> Result<&str, &str> {
        let first_key_value = accept_key_value(stream);
        if first_key_value.is_err() {
            return first_key_value;
        }
        first_key_value
            .and_then(accept_cb(','))
            .and_then(accept_key_values)
            .or_else(|advanced_stream| Ok(advanced_stream))
    }

    fn accept_object(stream: &str) -> Result<&str, &str> {
        accept(stream, '{')
            .and_then(|advanced_stream| accept_key_values(advanced_stream)
                .or_else(accept_whitespace))
            .and_then(accept_cb('}'))
    }

    fn accept_hex(stream: &str) -> Result<&str, &str> {
        accept_digit(stream)
            .or_else(accept_ignoring_case_cb('a'))
            .or_else(accept_ignoring_case_cb('b'))
            .or_else(accept_ignoring_case_cb('c'))
            .or_else(accept_ignoring_case_cb('d'))
            .or_else(accept_ignoring_case_cb('e'))
            .or_else(accept_ignoring_case_cb('f'))
    }

    fn accept_unicode(stream: &str) -> Result<&str, &str> {
        accept(stream, 'u')
            .and_then(accept_hex)
            .and_then(accept_hex)
            .and_then(accept_hex)
            .and_then(accept_hex)
    }

    fn accept_control_characters(stream: &str) -> Result<&str, &str> {
        accept(stream, '\\')
            .and_then(|advanced_stream| accept(advanced_stream, '\\')
                .or_else(accept_cb('/'))
                .or_else(accept_cb('b'))
                .or_else(accept_cb('b'))
                .or_else(accept_cb('n'))
                .or_else(accept_cb('r'))
                .or_else(accept_cb('t'))
                .or_else(accept_unicode))
    }

    fn accept_symbol(stream: &str) -> Result<&str, &str> {
        // TODO: also accept any codepoint except " and \
        accept_control_characters(stream)
    }

    fn accept_symbols(stream: &str) -> Result<&str, &str> {
        accept_symbol(stream)
            .and_then(accept_symbols)
            .or_else(|advanced_stream| Ok(advanced_stream))
    }

    fn accept_string(stream: &str) -> Result<&str, &str> {
        accept(stream, '"')
            .and_then(accept_symbols)
            .and_then(accept_cb('"'))
    }

    fn accept_value(stream: &str) -> Result<&str, &str> {
        accept_whitespace(stream)
            .and_then(accept_string)
            .or_else(accept_number)
            .or_else(accept_object)
            .or_else(accept_array)
            .or_else(accept_true)
            .or_else(accept_false)
            .or_else(accept_null)
            .and_then(accept_whitespace)
    }

    fn single_json(file: &String) -> Result<JsonValue, ParseError> {
        match File::open(file) {
            Ok(fp) => {
                let r = BufReader::new(fp);
                for (row, line) in r.lines().enumerate() {
                    match line {
                        Ok(string) => for (col, ch) in string.chars().enumerate() {
                            JsonValue::NULL
                        },
                        Err(e) => return ParseError {
                            row: row,
                            col: 0,
                            msg: format!("Unexpected EOF: {}", e.to_string())
                        }
                    }
                }
            },
            Err(e) => ParseError {
                row: 0,
                col: 0,
                msg: format!("Unable to open file {}: {}", file, e.to_string())
            }
        }
    }
}

mod stat {
    struct Stats {
        root_type: String,
    }

    pub fn single_json(file: &String) -> Result<Stats, std::io::Error> {
        let parse_result = parse_single_json(file);
        Ok(Stats {
            root_type: "not implemented yet!".to_string()
        })
    }

    pub fn multiple_jsons(files: &[String]) -> Result<Stats, std::io::Error> {
        for file in files {
            let stat_result = single_json(&file);
            return stat_result;
        }
        Ok(Stats {
            root_type: "not implemented yet!".to_string()
        })
    }
}

