use std::collections::HashMap;

use serde::{
    de::{self, DeserializeSeed, Unexpected, Visitor},
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Serialize,
};

use crate::stac::{DataType, DataVal, Struct};

#[derive(Serialize, Deserialize)]
pub struct ProviderSchema {
    pub functions: Vec<String>,
}

#[derive(Serialize)]
pub struct DMCLRPC<'a> {
    pub id: (usize, usize, usize),
    pub params: Vec<TypeAndVal<'a>>,
}

pub enum Expecting<T>
where
    T: Serialize,
{
    Found(T),
    Waiting,
}

impl<T> Serialize for Expecting<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Found(val) => Serialize::serialize(val, serializer),
            Self::Waiting => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("$waiting", &true)?;
                map.end()
            }
        }
    }
}

#[derive(Clone)]
pub struct TypeAndVal<'a> {
    pub val: DataVal,
    pub typ: DataType,
    pub user_structs: &'a HashMap<String, Struct>,
}

impl Serialize for TypeAndVal<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.typ {
            DataType::Integer => {
                let i = self.val.clone().into_integer().unwrap();
                serializer.serialize_i64(i)
            }
            DataType::Float => {
                let f = self.val.clone().into_float().unwrap();
                serializer.serialize_f64(f)
            }
            DataType::Bool => {
                let b = self.val.clone().into_bool().unwrap();
                serializer.serialize_bool(b)
            }
            DataType::String => {
                let s = self.val.clone().into_string().unwrap();
                serializer.serialize_str(&s)
            }
            DataType::Array(el_typ) => {
                let arr = self.val.clone().into_compound().unwrap();

                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for el in arr {
                    seq.serialize_element(&TypeAndVal {
                        val: el,
                        typ: *el_typ.clone(),
                        user_structs: &self.user_structs,
                    })?;
                }

                seq.end()
            }
            DataType::Struct(struct_name) => {
                let struct_struct = self.user_structs.get(struct_name).unwrap().clone();
                let arr = self.val.clone().into_compound().unwrap();

                let mut map = serializer.serialize_map(Some(arr.len()))?;
                for (idx, val) in arr.iter().enumerate() {
                    map.serialize_entry(
                        struct_struct
                            .names
                            .iter()
                            .find(|kv| *kv.1 == idx)
                            .unwrap()
                            .0,
                        &TypeAndVal {
                            val: val.clone(),
                            typ: struct_struct.types[idx].clone(),
                            user_structs: &self.user_structs,
                        },
                    )?;
                }

                map.end()
            }
            DataType::Waiting => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("$waiting", &true)?;
                map.end()
            }
        }
    }
}

impl<'de> DeserializeSeed<'de> for TypeAndVal<'_> {
    type Value = DataVal;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(TypeAndValVisitor {
            user_structs: self.user_structs,
            typ: self.typ,
        })
    }
}

struct TypeAndValVisitor<'a> {
    user_structs: &'a HashMap<String, Struct>,
    typ: DataType,
}

impl<'de> Visitor<'de> for TypeAndValVisitor<'_> {
    type Value = DataVal;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(&format!("a DataVal of type {:?}", self.typ))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.typ {
            DataType::Integer => Ok(DataVal::Integer(v)),
            DataType::Float => Ok(DataVal::Float(v as f64)),
            _ => Err(de::Error::invalid_type(Unexpected::Signed(v), &self)),
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.typ {
            DataType::Integer => Ok(DataVal::Integer(v as i64)),
            DataType::Float => Ok(DataVal::Float(v as f64)),
            _ => Err(de::Error::invalid_type(Unexpected::Unsigned(v), &self)),
        }
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.typ {
            DataType::Integer => Ok(DataVal::Integer(v as i64)),
            DataType::Float => Ok(DataVal::Float(v)),
            _ => Err(de::Error::invalid_type(Unexpected::Float(v), &self)),
        }
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.typ {
            DataType::Bool => Ok(DataVal::Bool(v)),
            _ => Err(de::Error::invalid_type(Unexpected::Bool(v), &self)),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.typ {
            DataType::String => Ok(DataVal::String(v.to_string())),
            _ => Err(de::Error::invalid_type(Unexpected::Str(v), &self)),
        }
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        match self.typ {
            DataType::Array(el_type) => {
                let mut arr = vec![];
                while let Some(el) = seq.next_element_seed(TypeAndVal {
                    val: DataVal::Bool(false),
                    typ: *el_type.clone(),
                    user_structs: self.user_structs,
                })? {
                    arr.push(el);
                }

                Ok(DataVal::Compound(arr))
            }
            _ => Err(de::Error::invalid_type(Unexpected::Seq, &self)),
        }
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        match self.typ {
            DataType::Struct(struct_name) => {
                let stru = &self.user_structs[&struct_name];

                let mut arr = vec![DataVal::Bool(false); stru.types.len()];
                while let Some(key) = map.next_key::<String>()? {
                    // If any key in this map is waiting (there should only be one)
                    // then this entire object is a single DataVal::Waiting.
                    if key == "$waiting" {
                        map.next_value::<serde::de::IgnoredAny>()?;
                        return Ok(DataVal::Waiting);
                    }

                    // Discard unknown keys
                    if stru.names.get(&key).is_none() {
                        map.next_value::<serde::de::IgnoredAny>()?;
                        continue;
                    }

                    let val = map.next_value_seed(TypeAndVal {
                        val: DataVal::Bool(false),
                        typ: stru.types[stru.names[&key]].clone(),
                        user_structs: self.user_structs,
                    })?;
                    arr[stru.names[&key]] = val;
                }

                Ok(DataVal::Compound(arr))
            }
            _ => {
                // this could be waiting
                while let Some((k, v)) = map.next_entry::<String, bool>()? {
                    if k != "$waiting" || !v {
                        return Err(de::Error::custom(
                            "DataVal must be ::Waiting, but has wrong key",
                        ));
                    }
                }

                Ok(DataVal::Waiting)
            }
        }
    }
}

pub struct ExternReturns<'a> {
    pub user_structs: &'a HashMap<String, Struct>,
    pub types: Vec<DataType>,
}

impl<'de> DeserializeSeed<'de> for ExternReturns<'_> {
    type Value = Vec<DataVal>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ExternReturnsVisitor {
            user_structs: self.user_structs,
            types: self.types,
        })
    }
}

struct ExternReturnsVisitor<'a> {
    user_structs: &'a HashMap<String, Struct>,
    types: Vec<DataType>,
}

impl<'de> Visitor<'de> for ExternReturnsVisitor<'_> {
    type Value = Vec<DataVal>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an array of DataVals")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut arr = vec![];
        let mut i = 0;
        while let Some(el) = seq.next_element_seed(TypeAndVal {
            val: DataVal::Bool(false),
            typ: self.types[i].clone(),
            user_structs: self.user_structs,
        })? {
            arr.push(el);
            i += 1;

            if i == self.types.len() {
                break;
            }
        }

        Ok(arr)
    }
}
