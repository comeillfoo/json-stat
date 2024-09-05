use std::{collections::HashMap, fs};


pub struct ParseError {
    pub row: usize,
    pub col: usize,
    pub msg: String
}

#[derive(Debug)]
pub enum JsonValue {
    STRING(String),
    NUMBER(f64),
    OBJECT(HashMap<String, JsonValue>),
    ARRAY(Vec<JsonValue>),
    TRUE,
    FALSE,
    NULL,
    KEYVALUE((String, Box<JsonValue>))
}

impl Clone for JsonValue {
    fn clone(&self) -> Self {
        match self {
            JsonValue::STRING(line) => JsonValue::STRING(line.clone()),
            JsonValue::NUMBER(num) => JsonValue::NUMBER(*num),
            JsonValue::OBJECT(obj) => JsonValue::OBJECT(obj.clone()),
            JsonValue::ARRAY(arr) => JsonValue::ARRAY(arr.clone()),
            JsonValue::TRUE => JsonValue::TRUE,
            JsonValue::FALSE => JsonValue::FALSE,
            JsonValue::NULL => JsonValue::NULL,
            JsonValue::KEYVALUE((key, value)) => JsonValue::KEYVALUE((key.clone(), Box::new((**value).clone()))),
        }
    }
}

struct JsonFragment<'a> {
    stream: &'a str,
    raw: Vec<char>,
    value: JsonValue
}

fn accept_common(mut frag: JsonFragment, expected: char, should_ignore: bool) -> Result<JsonFragment, JsonFragment> {
    match frag.stream.chars().next() {
        Some(actual) => if actual == expected {
            if ! should_ignore { frag.raw.push(expected); }
            Ok(JsonFragment {
                stream: &frag.stream[1..],
                raw: frag.raw,
                value: frag.value
            })
        } else {
            Err(frag)
        },
        None => Err(frag)
    }
}

fn accept(frag: JsonFragment, expected: char) -> Result<JsonFragment, JsonFragment> {
    accept_common(frag, expected, false)
}

fn accept_delimiter(frag: JsonFragment, expected: char) -> Result<JsonFragment, JsonFragment> {
    accept_common(frag, expected, true)
}

fn just_accept(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    Ok(frag)
}

fn accept_cb(expected: char) -> impl FnOnce(JsonFragment) -> Result<JsonFragment, JsonFragment> {
    move |frag: JsonFragment| accept(frag, expected)
}

fn accept_delimiter_cb(expected: char) -> impl FnOnce(JsonFragment) -> Result<JsonFragment, JsonFragment> {
    move |frag: JsonFragment| accept_delimiter(frag, expected)
}

fn accept_ignoring_case(frag: JsonFragment, expected: char) -> Result<JsonFragment, JsonFragment> {
    accept(frag, expected.to_ascii_lowercase())
        .or_else(accept_cb(expected.to_ascii_uppercase()))
}

fn accept_ignoring_case_cb(expected: char) -> impl FnOnce(JsonFragment) -> Result<JsonFragment, JsonFragment> {
    move |frag: JsonFragment| accept_ignoring_case(frag, expected)
}

fn accept_whitespace(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_delimiter(frag, ' ')
        .or_else(accept_delimiter_cb('\n'))
        .or_else(accept_delimiter_cb('\r'))
        .or_else(accept_delimiter_cb('\t'))
        .or_else(just_accept)
}

fn accept_true(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let r_frag = accept(frag, 't')
        .and_then(accept_cb('r'))
        .and_then(accept_cb('u'))
        .and_then(accept_cb('e'))?;
    Ok(JsonFragment {
        stream: r_frag.stream,
        raw: vec![],
        value: JsonValue::TRUE
    })
}

fn accept_false(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let r_frag = accept(frag, 'f')
        .and_then(accept_cb('a'))
        .and_then(accept_cb('l'))
        .and_then(accept_cb('s'))
        .and_then(accept_cb('e'))?;
    Ok(JsonFragment {
        stream: r_frag.stream,
        raw: vec![],
        value: JsonValue::FALSE
    })
}

fn accept_null(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let r_frag = accept(frag, 'n')
        .and_then(accept_cb('u'))
        .and_then(accept_cb('l'))
        .and_then(accept_cb('l'))?;
    return Ok(JsonFragment {
        stream: r_frag.stream,
        raw: vec![],
        value: JsonValue::NULL
    })
}

fn accept_nonzero(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept(frag, '1')
        .or_else(accept_cb('2'))
        .or_else(accept_cb('3'))
        .or_else(accept_cb('4'))
        .or_else(accept_cb('5'))
        .or_else(accept_cb('6'))
        .or_else(accept_cb('7'))
        .or_else(accept_cb('8'))
        .or_else(accept_cb('9'))
}

fn accept_digit(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_nonzero(frag).or_else(accept_cb('0'))
}

fn accept_digits(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let digit = accept_digit(frag);
    if digit.is_err() {
        return digit;
    }
    digit.and_then(accept_digits)
        .or_else(just_accept)
}

fn accept_exponent(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_ignoring_case(frag, 'e')
        .and_then(|r_frag| accept(r_frag, '+')
            .or_else(accept_cb('-'))
            .or_else(just_accept))
        .and_then(accept_digits)
}

fn accept_fraction(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept(frag, '.')
        .and_then(accept_digits)
}

fn accept_integer(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept(frag, '-')
        .or_else(just_accept)
        .and_then(accept_cb('0'))
        .or_else(|r_frag| accept_nonzero(r_frag)
            .and_then(accept_digits)
            .or_else(just_accept))
}

fn accept_number(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let frag_integer = accept_integer(JsonFragment {
        stream: frag.stream,
        raw: frag.raw.clone(),
        value: frag.value.clone()
    })?;
    let frag_number = accept_fraction(frag_integer)
        .or_else(just_accept)
        .and_then(accept_exponent)
        .or_else(just_accept)?;
    match frag_number.raw
                .into_iter()
                .collect::<String>().parse::<f64>() {
        Ok(number) => Ok(JsonFragment {
            stream: frag_number.stream,
            raw: vec![],
            value: JsonValue::NUMBER(number)
        }),
        Err(_) => Err(frag)
    }
}

fn accept_values(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let frag_value = frag.value.clone();
    if let JsonValue::ARRAY(mut arr) = frag_value {
        let first_value = accept_value(JsonFragment {
            stream: frag.stream,
            raw: frag.raw,
            value: frag.value
        })?;
        arr.push(first_value.value);
        return accept_delimiter(JsonFragment {
            stream: first_value.stream,
            raw: vec![],
            value: JsonValue::ARRAY(arr)
        }, ',').and_then(accept_values)
            .or_else(just_accept);
    }
    Err(frag)
}

fn accept_array(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_delimiter(frag, '[')
        // .and_then(accept_whitespace)
        .and_then(|r_frag| accept_values(JsonFragment {
                stream: r_frag.stream,
                raw: r_frag.raw,
                value: JsonValue::ARRAY(vec![])
        }))
        .and_then(accept_delimiter_cb(']'))
}

fn accept_key_value(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let key_frag = accept_whitespace(JsonFragment {
        stream: frag.stream,
        raw: frag.raw.clone(),
        value: frag.value.clone()
    }).and_then(accept_string)
        .and_then(accept_whitespace)
        .and_then(accept_delimiter_cb(':'))?;
    let value_frag = accept_value(JsonFragment {
        stream: key_frag.stream,
        raw: vec![],
        value: frag.value.clone()
    })?;
    let key_frag_value = key_frag.value;
    if let JsonValue::STRING(key) = key_frag_value {
        return Ok(JsonFragment {
            stream: value_frag.stream,
            raw: value_frag.raw,
            value: JsonValue::KEYVALUE((key, Box::new(value_frag.value)))
        });
    }
    Err(frag)
}

fn accept_key_values(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let frag_value = frag.value.clone();
    if let JsonValue::OBJECT(mut obj) = frag_value.clone() {
        let first_key_value = accept_key_value(JsonFragment {
            stream: frag.stream,
            raw: frag.raw.clone(),
            value: frag_value
        })?;

        if let JsonValue::KEYVALUE((key, value)) = first_key_value.value {
            obj.insert(key, *value);
            return accept_delimiter(JsonFragment {
                stream: first_key_value.stream,
                raw: first_key_value.raw,
                value: JsonValue::OBJECT(obj)
            }, ',').and_then(accept_key_values)
                .or_else(|r_frag| Ok(r_frag));
        }
    }
    Err(frag)
}

fn accept_object(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_delimiter(frag, '{')
        // .and_then(accept_whitespace)
        .and_then(|r_frag| accept_key_values(JsonFragment {
            stream: r_frag.stream,
            raw: r_frag.raw,
            value: JsonValue::OBJECT(HashMap::new())
        }))
        .and_then(accept_delimiter_cb('}'))
}

fn accept_hex(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_digit(frag)
        .or_else(accept_ignoring_case_cb('a'))
        .or_else(accept_ignoring_case_cb('b'))
        .or_else(accept_ignoring_case_cb('c'))
        .or_else(accept_ignoring_case_cb('d'))
        .or_else(accept_ignoring_case_cb('e'))
        .or_else(accept_ignoring_case_cb('f'))
}

fn accept_unicode(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept(frag, 'u')
        .and_then(accept_hex)
        .and_then(accept_hex)
        .and_then(accept_hex)
        .and_then(accept_hex)
}

fn accept_control_characters(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept(frag, '\\')
        .and_then(|r_frag| accept(r_frag, '\\')
            .or_else(accept_cb('/'))
            .or_else(accept_cb('b'))
            .or_else(accept_cb('b'))
            .or_else(accept_cb('n'))
            .or_else(accept_cb('r'))
            .or_else(accept_cb('t'))
            .or_else(accept_unicode))
}

fn accept_symbol(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    match frag.stream.chars().next() {
        Some(actual) => if actual != '"' && actual != '\\' {
            accept(frag, actual)
        } else {
            accept_control_characters(frag)
        },
        None => Err(frag)
    }
}

fn accept_symbols(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_symbol(frag)
        .and_then(accept_symbols)
        .or_else(|r_frag| Ok(r_frag))
}

fn accept_string(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    let frag_string = accept_delimiter(frag, '"')
        .and_then(accept_symbols)
        .and_then(accept_delimiter_cb('"'))?;
    Ok(JsonFragment {
        stream: frag_string.stream,
        raw: vec![],
        value: JsonValue::STRING(frag_string.raw
            .into_iter().collect::<String>())
    })
}

fn accept_value(frag: JsonFragment) -> Result<JsonFragment, JsonFragment> {
    accept_whitespace(frag)
        .and_then(accept_string)
        .or_else(accept_number)
        .or_else(accept_object)
        .or_else(accept_array)
        .or_else(accept_true)
        .or_else(accept_false)
        .or_else(accept_null)
        .and_then(accept_whitespace)
}

pub fn single_json(file: &String) -> Result<Option<JsonValue>, ParseError> {
    match fs::read_to_string(file) {
        Ok(content) => match accept_value(JsonFragment {
            stream: &content,
            raw: vec![],
            value: JsonValue::NULL
        }) {
            Ok(frag) => Ok(Some(frag.value)),
            Err(_) => Ok(None)
        },
        Err(e) => Err(ParseError {
            row: 0,
            col: 0,
            msg: format!("Unable to open file {}: {}", file, e.to_string())
        })
    }
}
