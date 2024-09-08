use std::collections::{HashSet, HashMap};

const JSON_TYPES_NAMES: [&'static str; 8] = [
    "string", "number", "object", "array",
    "true", "false", "null", "key-value"
];

struct JsonObjectStats {
    inner_stats: HashMap<String, JsonComplexTypeStats>,
    keys: HashSet<String>,
    likely_mandatory: HashSet<&str>
}

enum JsonSpecificTypeStats {
    OBJECT(JsonObjectStats),
    ARRAY(Vec<JsonComplexTypeStats>)
}

struct JsonComplexTypeStats {
    values_types: HashSet<&str>,
    numbers: HashSet<f64>,
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
        JsonValue::KEYVALUE => 7,
        _ => unreachable!()
    };
    JSON_TYPES_NAMES[idx]
}

fn is_complex_type(json: &JsonValue) -> bool {
    match json {
        JsonValue::OBJECT(_) => true,
        JsonValue::ARRAY(_) => true,
        _ => false
    }
}

impl JsonComplexTypeStats {
    pub fn object() -> Self {
        Self {
            values_types: HashSet::new(),
            numbers: HashSet::new(),
            strings: HashSet::new(),
            type_stats: JsonSpecificTypeStats::OBJECT(JsonObjectStats {
                inner_stats: HashMap::new(),
                keys: HashSet::new(),
                likely_mandatory: HashSet::new()
            })
        }
    }

    pub fn array() -> Self {
        Self {
            values_types: HashSet::new(),
            numbers: HashSet::new(),
            strings: HashSet::new(),
            type_stats: JsonSpecificTypeStats::ARRAY(vec![])
        }
    }

    pub fn merge_common_stats(&mut self, value: &JsonValue) {
        self.values_types.insert(stringify_json(&value));
        if let JsonValue::NUMBER(num) = *value {
            self.numbers.insert(num);
        }
        if let JsonValue::STRING(line) = *value {
            self.strings.insert(line);
        }
    }

    pub fn stat_object(object: HashMap<String, Box<JsonValue>>) -> Self {
        let mut stats = JsonComplexTypeStats::object();
        for (key, value) in object {
            stats.merge_common_stats(&*value);
            if let JsonSpecificTypeStats::OBJECT(ref mut obj_stats) = stats.type_stats {
                obj_stats.keys.insert(key);
                if is_complex_type(&*value) {
                    obj_stats.inner_stats.insert(key, JsonComplexTypeStats::stat(*value));
                }
            }
        }
        stats
    }

    pub fn stat_array(array: Vec<JsonValue>) -> Self {
        let mut stats = JsonComplexTypeStats::array();
        for value in array {
            stats.merge_common_stats(&value);
            if let JsonSpecificTypeStats::ARRAY(ref mut arr_stats) = stats.type_stats {
                if is_complex_type(&value) {
                    arr_stats.push(JsonComplexTypeStats::stat(value));
                }
            }
        }
        stats
    }

    pub fn stat(json: JsonValue) -> Self {
        match json {
            JsonValue::OBJECT(object) => JsonComplexTypeStats::stat_object(object),
            JsonValue::ARRAY(array) => JsonComplexTypeStats::stat_array(array),
            _ => unreachable!() // handle non complex types
        }
    }
}

