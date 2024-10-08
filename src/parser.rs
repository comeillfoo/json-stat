use std::{cell::RefCell, collections::HashMap, fs};


pub struct ParseError {
    pub row: usize,
    pub col: usize,
    pub msg: String
}

#[derive(Debug, PartialEq)]
pub enum JsonValue {
    STRING(String),
    NUMBER(f64),
    OBJECT(HashMap<String, Box<JsonValue>>),
    ARRAY(Vec<JsonValue>),
    TRUE,
    FALSE,
    NULL,
    KEYVALUE((String, Box<JsonValue>))
}

thread_local! {
    static COLUMN: RefCell<usize> = RefCell::new(0);
    static ROW: RefCell<usize> = RefCell::new(0);
    static CHAR_STREAM: RefCell<&'static str> = RefCell::new("");
    static RAW_CHARS: RefCell<Vec<char>> = RefCell::new(vec![]);
}

fn get_next_char() -> char {
    let next = CHAR_STREAM.with(|rc| rc.borrow().chars().next());
    if let Some(symbol) = next {
        return symbol;
    }
    '\0'
}

fn accept_common(jval: JsonValue, expected: char, should_ignore: bool) -> Result<JsonValue, JsonValue> {
    let actual = get_next_char();
    if actual == '\0' {
        return Err(jval);
    }

    if actual == expected {
        COLUMN.with(|rc| { *rc.borrow_mut() += 1; });
        if ! should_ignore {
            RAW_CHARS.with(|rc| { rc.borrow_mut().push(expected); });
        } else if actual == '\n' {
            ROW.with(|rc| { *rc.borrow_mut() += 1; });
            COLUMN.with(|rc| { *rc.borrow_mut() = 0; })
        }
        CHAR_STREAM.with(|rc| { rc.replace_with(|&mut old| &old[1..]); });

        Ok(jval)
    } else {
        Err(jval)
    }
}

fn accept(jval: JsonValue, expected: char) -> Result<JsonValue, JsonValue> {
    accept_common(jval, expected, false)
}

fn accept_delimiter(jval: JsonValue, expected: char) -> Result<JsonValue, JsonValue> {
    accept_common(jval, expected, true)
}

fn just_accept(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    Ok(jval)
}

fn accept_cb(expected: char) -> impl FnOnce(JsonValue) -> Result<JsonValue, JsonValue> {
    move |jval: JsonValue| accept(jval, expected)
}

fn accept_delimiter_cb(expected: char) -> impl FnOnce(JsonValue) -> Result<JsonValue, JsonValue> {
    move |jval: JsonValue| accept_delimiter(jval, expected)
}

fn accept_ignoring_case(jval: JsonValue, expected: char) -> Result<JsonValue, JsonValue> {
    accept(jval, expected.to_ascii_lowercase())
        .or_else(accept_cb(expected.to_ascii_uppercase()))
}

fn accept_ignoring_case_cb(expected: char) -> impl FnOnce(JsonValue) -> Result<JsonValue, JsonValue> {
    move |jval: JsonValue| accept_ignoring_case(jval, expected)
}

fn accept_whitespaces(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_delimiter(jval, ' ')
        .or_else(accept_delimiter_cb('\n'))
        .or_else(accept_delimiter_cb('\r'))
        .or_else(accept_delimiter_cb('\t'))
}

pub fn accept_whitespace(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_whitespaces(jval)
        .and_then(accept_whitespace)
        .or_else(just_accept)
}

pub fn accept_true(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, 't')
        .and_then(accept_cb('r'))
        .and_then(accept_cb('u'))
        .and_then(accept_cb('e'))
        .and(Ok(JsonValue::TRUE))
}

pub fn accept_false(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, 'f')
        .and_then(accept_cb('a'))
        .and_then(accept_cb('l'))
        .and_then(accept_cb('s'))
        .and_then(accept_cb('e'))
        .and(Ok(JsonValue::FALSE))
}

pub fn accept_null(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, 'n')
        .and_then(accept_cb('u'))
        .and_then(accept_cb('l'))
        .and_then(accept_cb('l'))
        .and(Ok(JsonValue::NULL))
}

fn accept_nonzero(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, '1')
        .or_else(accept_cb('2'))
        .or_else(accept_cb('3'))
        .or_else(accept_cb('4'))
        .or_else(accept_cb('5'))
        .or_else(accept_cb('6'))
        .or_else(accept_cb('7'))
        .or_else(accept_cb('8'))
        .or_else(accept_cb('9'))
}

fn accept_digit(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_nonzero(jval).or_else(accept_cb('0'))
}

fn accept_digits(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    let digit = accept_digit(jval);
    if digit.is_err() {
        return digit;
    }
    digit.and_then(accept_digits)
        .or_else(just_accept)
}

fn accept_exponent(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_ignoring_case(jval, 'e')
        .and_then(|r_jval| accept(r_jval, '+')
            .or_else(accept_cb('-'))
            .or_else(just_accept))
        .and_then(accept_digits)
}

fn accept_fraction(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, '.')
        .and_then(accept_digits)
}

fn accept_integer(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, '-')
        .or_else(just_accept)
        .and_then(accept_cb('0'))
        .or_else(|r_jval| accept_nonzero(r_jval)
            .and_then(accept_digits)
            .or_else(just_accept))
}

pub fn accept_number(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    let _jval_integer = accept_integer(JsonValue::NULL)?;
    let _jval_number = accept_fraction(JsonValue::NULL)
        .or_else(just_accept)
        .and_then(accept_exponent)
        .or_else(just_accept)?;
    let maybe_parsed = RAW_CHARS.with(
        |rc| rc.borrow().iter().collect::<String>().parse::<f64>());

    match maybe_parsed {
        Ok(number) => {
            RAW_CHARS.with(|rc| rc.borrow_mut().clear());
            Ok(JsonValue::NUMBER(number))
        },
        Err(_) => Err(jval)
    }
}

fn accept_values(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    if let JsonValue::ARRAY(mut arr) = jval {
        let first_value = accept_value(JsonValue::NULL)?;
        arr.push(first_value);
        return accept_delimiter(JsonValue::ARRAY(arr), ',')
            .and_then(accept_values)
            .or_else(just_accept);
    }
    Err(jval)
}

pub fn accept_array(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_delimiter(jval, '[')
        .and_then(|_r_jval| accept_values(JsonValue::ARRAY(vec![])))
        .and_then(accept_delimiter_cb(']'))
}

fn accept_key_value(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    let key_jval = accept_whitespace(JsonValue::NULL)
        .and_then(accept_string)
        .and_then(accept_whitespace)
        .and_then(accept_delimiter_cb(':'))?;
    let value_jval = accept_value(JsonValue::NULL)?;
    if let JsonValue::STRING(key) = key_jval {
        return Ok(JsonValue::KEYVALUE((key, Box::new(value_jval))));
    }
    Err(jval)
}

fn accept_key_values(mut jval: JsonValue) -> Result<JsonValue, JsonValue> {
    if let JsonValue::OBJECT(ref mut obj) = jval {
        let first_key_value = accept_key_value(JsonValue::NULL)?;
        if let JsonValue::KEYVALUE((key, value)) = first_key_value {
            obj.insert(key, value);
            return accept_delimiter(jval, ',')
                .and_then(accept_key_values)
                .or_else(|r_frag| Ok(r_frag));
        }
    }
    Err(jval)
}

pub fn accept_object(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_delimiter(jval, '{')
        .and_then(|_r_jval| accept_key_values(JsonValue::OBJECT(HashMap::new())))
        .and_then(accept_delimiter_cb('}'))
}

fn accept_hex(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_digit(jval)
        .or_else(accept_ignoring_case_cb('a'))
        .or_else(accept_ignoring_case_cb('b'))
        .or_else(accept_ignoring_case_cb('c'))
        .or_else(accept_ignoring_case_cb('d'))
        .or_else(accept_ignoring_case_cb('e'))
        .or_else(accept_ignoring_case_cb('f'))
}

fn accept_unicode(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, 'u')
        .and_then(accept_hex)
        .and_then(accept_hex)
        .and_then(accept_hex)
        .and_then(accept_hex)
}

fn accept_control_characters(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept(jval, '\\')
        .and_then(|r_jval| accept(r_jval, '"')
            .or_else(accept_cb('\\'))
            .or_else(accept_cb('/'))
            .or_else(accept_cb('b'))
            .or_else(accept_cb('f'))
            .or_else(accept_cb('n'))
            .or_else(accept_cb('r'))
            .or_else(accept_cb('t'))
            .or_else(accept_unicode))
}

fn accept_symbol(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    let actual = get_next_char();
    if actual == '\0' {
        return Err(jval);
    }

    if actual != '"' && actual != '\\' {
        accept(jval, actual)
    } else {
        accept_control_characters(jval)
    }
}

fn accept_symbols(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_symbol(jval)
        .and_then(accept_symbols)
        .or_else(|r_jval| Ok(r_jval))
}

pub fn accept_string(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    let _jval_string = accept_delimiter(jval, '"')
        .and_then(accept_symbols)
        .and_then(accept_delimiter_cb('"'))?;
    let res = RAW_CHARS.with(|rc| rc.borrow().iter().collect::<String>());
    RAW_CHARS.with(|rc| rc.borrow_mut().clear());
    Ok(JsonValue::STRING(res))
}

pub fn accept_value(jval: JsonValue) -> Result<JsonValue, JsonValue> {
    accept_whitespace(jval)
        .and_then(accept_string)
        .or_else(accept_number)
        .or_else(accept_object)
        .or_else(accept_array)
        .or_else(accept_true)
        .or_else(accept_false)
        .or_else(accept_null)
        .and_then(accept_whitespace)
}

pub fn prepare_environment(content: String) {
    COLUMN.with(|rc| { *rc.borrow_mut() = 0; });
    ROW.with(|rc| { *rc.borrow_mut() = 0; });
    RAW_CHARS.with(|rc| rc.borrow_mut().clear());
    CHAR_STREAM.with(|rc| { rc.replace(Box::leak(content.into_boxed_str())); });
}

pub fn single_json(file: &String) -> Result<Option<JsonValue>, ParseError> {
    match fs::read_to_string(file) {
        Ok(content) => {
            prepare_environment(content);
            match accept_value(JsonValue::NULL) {
                Ok(jval) => Ok(Some(jval)),
                Err(_) => Err(ParseError {
                    row: ROW.with(|rc| *rc.borrow()),
                    col: COLUMN.with(|rc| *rc.borrow()),
                    msg: format!("unexpected symbol \'{}\'", get_next_char())
                })
            }
        },
        Err(e) => Err(ParseError {
            row: 0,
            col: 0,
            msg: format!("unable to open/read file: {}", e.to_string())
        })
    }
}
