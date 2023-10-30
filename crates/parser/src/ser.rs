use serde::{
    ser::{Serialize, SerializeMap, SerializeSeq},
    Serializer,
};

use crate::value::{Number, SpannedValue, Value};

impl Serialize for SpannedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Value::Number(Number::Float(num)) => serializer.serialize_f64(*num),
            Value::Number(Number::PosInt(num)) => serializer.serialize_u64(*num),

            Value::Number(Number::NegInt(num)) => serializer.serialize_i64(*num),
            Value::String(str) => serializer.serialize_str(str),
            Value::Bool(bool) => serializer.serialize_bool(*bool),
            Value::Array(array) => {
                let mut seq = serializer.serialize_seq(Some(array.len()))?;

                for v in array {
                    seq.serialize_element(v)?;
                }

                seq.end()
            }
            Value::Object(obj) => {
                let mut map = serializer.serialize_map(Some(obj.len()))?;

                for (k, v) in obj {
                    map.serialize_entry(k, v)?;
                }

                map.end()
            }
        }
    }
}
