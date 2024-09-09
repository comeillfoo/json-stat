use std::collections::{HashSet, HashMap, BinaryHeap};
use std::cmp::{Reverse, Ordering};

use crate::parser::JsonValue;

const JSON_TYPES_NAMES: [&'static str; 8] = [
    "string", "number", "object", "array",
    "true", "false", "null", "key-value"
];

#[derive(PartialEq)]
struct NonNan(f64);

impl Eq for NonNan {}

impl PartialOrd for NonNan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for NonNan {
    fn cmp(&self, other: &NonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

struct JsonNumbersStats {
    limit: usize,
    minimums: BinaryHeap<NonNan>,
    maximums: BinaryHeap<Reverse<NonNan>>,
    sum: f64,
    number: usize
}

struct JsonArrayStats {
    inner_arrays_stats: Option<JsonComplexTypeStats>,
    inner_objects_stats: Option<JsonComplexTypeStats>
}

struct JsonObjectStats {
    primitives_keys: HashSet<String>,
    complex_stats: HashMap<String, JsonComplexTypeStats>,
    nonobligatory: HashSet<String>
}

enum JsonSpecificTypeStats {
    ARRAY(Box<JsonArrayStats>),
    OBJECT(Box<JsonObjectStats>)
}

struct JsonComplexTypeStats {
    values_types: HashSet<&'static str>,
    numbers: JsonNumbersStats,
    strings: HashSet<String>,
    type_stats: JsonSpecificTypeStats
}

fn stringify_json(json: &JsonValue) -> &str {
    let idx = match json {
        JsonValue::STRING(_) => 0,
        JsonValue::NUMBER(_) => 1,
        JsonValue::OBJECT(_) => 2,
        JsonValue::ARRAY(_) => 3,
        JsonValue::TRUE => 4,
        JsonValue::FALSE => 5,
        JsonValue::NULL => 6,
        JsonValue::KEYVALUE(_) => 7
    };
    JSON_TYPES_NAMES[idx]
}

fn is_array_type(json: &JsonValue) -> bool {
    match json {
        JsonValue::ARRAY(_) => true,
        _ => false
    }
}

fn is_object_type(json: &JsonValue) -> bool {
    match json {
        JsonValue::OBJECT(_) => true,
        _ => false
    }
}

fn is_complex_type(json: &JsonValue) -> bool {
    is_array_type(json) || is_object_type(json)
}

fn merge_as_array_stats(mut stats: JsonArrayStats, json_array: JsonValue) -> JsonArrayStats {
    assert!(is_array_type(&json_array));
    if let JsonValue::ARRAY(array) = json_array {
        for value in array {
            if is_array_type(&value) {
                stats.inner_arrays_stats = Some(match stats.inner_arrays_stats {
                    Some(prev) => prev.merge_stats(value),
                    None => JsonComplexTypeStats::from_json(value)
                });
                continue;
            }
            if is_object_type(&value) {
                stats.inner_objects_stats = Some(match stats.inner_objects_stats {
                    Some(prev) => prev.merge_stats(value),
                    None => JsonComplexTypeStats::from_json(value)
                });
            }
        }
    }
    stats
}

fn merge_as_object_stats(mut stats: JsonObjectStats, json_object: JsonValue) -> JsonObjectStats {
    assert!(is_object_type(&json_object));
    if let JsonValue::OBJECT(object) = json_object {
        for (key_mold, value) in object {
            if is_complex_type(value.as_ref()) {
                if stats.complex_stats.contains_key(&key_mold) {
                    stats.nonobligatory.insert(key_mold.clone());
                }
                let new = match stats.complex_stats.remove(&key_mold) {
                    Some(prev) => prev.merge_stats(*value),
                    None => JsonComplexTypeStats::from_json(*value)
                };
                stats.complex_stats.insert(key_mold, new);
                continue;
            }
            if stats.primitives_keys.insert(key_mold.clone()) {
                stats.nonobligatory.insert(key_mold.clone());
            }
        }
    }
    stats
}

impl JsonNumbersStats {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            minimums: BinaryHeap::new(),
            maximums: BinaryHeap::new(),
            sum: 0f64,
            number: 0
        }
    }

    pub fn add(&mut self, number: f64) {
        assert!(! number.is_nan());

        if self.minimums.len() == self.limit {
            self.minimums.pop();
        }
        self.minimums.push(NonNan(number));
        if self.maximums.len() == self.limit {
            self.maximums.pop();
        }
        self.maximums.push(Reverse(NonNan(number)));
        self.number += 1;
        self.sum += number;
    }
}

impl JsonComplexTypeStats {
    pub fn array() -> Self {
        Self {
            values_types: HashSet::new(),
            numbers: JsonNumbersStats::new(10),
            strings: HashSet::new(),
            type_stats: JsonSpecificTypeStats::ARRAY(Box::new(JsonArrayStats {
                inner_arrays_stats: None,
                inner_objects_stats: None
            }))
        }
    }

    pub fn object() -> Self {
        Self {
            values_types: HashSet::new(),
            numbers: JsonNumbersStats::new(10),
            strings: HashSet::new(),
            type_stats: JsonSpecificTypeStats::OBJECT(Box::new(JsonObjectStats {
                complex_stats: HashMap::new(),
                primitives_keys: HashSet::new(),
                nonobligatory: HashSet::new()
            }))
        }
    }

    pub fn is_array_type(&self) -> bool {
        match self.type_stats {
            JsonSpecificTypeStats::ARRAY(_) => true,
            _ => false
        }
    }

    pub fn is_object_type(&self) -> bool {
        match self.type_stats {
            JsonSpecificTypeStats::OBJECT(_) => true,
            _ => false
        }
    }

    fn merge_primitives_stats(mut self, value: JsonValue) -> Self {
        if let JsonValue::NUMBER(num) = value {
            self.numbers.add(num);
        }
        if let JsonValue::STRING(line) = value {
            self.strings.insert(line);
        }
        self
    }

    fn merge_complex_stats(mut self, value: JsonValue) -> Self {
        self.type_stats = match self.type_stats {
            JsonSpecificTypeStats::ARRAY(arr_stats) => JsonSpecificTypeStats::ARRAY(Box::new(merge_as_array_stats(*arr_stats, value))),
            JsonSpecificTypeStats::OBJECT(obj_stats) => JsonSpecificTypeStats::OBJECT(Box::new(merge_as_object_stats(*obj_stats, value)))
        };
        self
    }

    pub fn from_object(object: HashMap<String, Box<JsonValue>>) -> Self {
        let mut stats = Self::object();
        for (key, value) in object {
            stats.values_types.insert(stringify_json(&value));
            if let JsonSpecificTypeStats::OBJECT(ref mut obj_stats) = stats.type_stats {
                if is_complex_type(&value) {
                    let new = match obj_stats.complex_stats.remove(&key) {
                        Some(prev) => prev.merge_stats(*value),
                        None => Self::from_json(*value)
                    };
                    obj_stats.complex_stats.insert(key, new);
                    continue;
                }
                obj_stats.primitives_keys.insert(key);
                stats.merge_primitives_stats(*value);
            }
        }
        stats
    }

    pub fn from_array(array: Vec<JsonValue>) -> Self {
        let mut stats = Self::array();
        for value in array {
            stats.values_types.insert(stringify_json(&value));
            if let JsonSpecificTypeStats::ARRAY(ref mut arr_stats) = stats.type_stats {
                if is_object_type(&value) {
                    arr_stats.inner_objects_stats = Some(match arr_stats.inner_objects_stats {
                        Some(prev) => prev.merge_stats(value),
                        None => Self::from_json(value)
                    });
                    continue;
                }
                if is_array_type(&value) {
                    arr_stats.inner_arrays_stats = Some(match arr_stats.inner_arrays_stats {
                        Some(prev) => prev.merge_stats(value),
                        None => Self::from_json(value)
                    })
                }
            }
        }
        stats
    }

    fn is_complex_matches(&self, value: &JsonValue) -> bool {
        (self.is_array_type() && is_array_type(value))
        || (self.is_object_type() && is_object_type(value))
    }

    pub fn merge_stats(mut self, value: JsonValue) -> Self {
        if self.is_complex_matches(&value) {
            return self.merge_complex_stats(value);
        }
        self.values_types.insert(stringify_json(&value));
        self.merge_primitives_stats(value)
    }

    pub fn from_json(json: JsonValue) -> Self {
        if is_complex_type(&json) {
            return match json {
                JsonValue::OBJECT(object) => Self::from_object(object),
                JsonValue::ARRAY(array) => Self::from_array(array),
                _ => unreachable!()
            };
        }
        Self::from_array(vec![json])
    }
}

