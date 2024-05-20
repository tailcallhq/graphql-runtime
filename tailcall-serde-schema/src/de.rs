use serde::de::{self};

use crate::{
    schema::{self, Schema},
    value,
};

type FieldMap = [(String, Box<Schema>)];
type Value = crate::Value;

pub struct Deserialize<'de> {
    schema: &'de Schema,
}

impl<'de> Deserialize<'de> {
    pub fn new(schema: &'de Schema) -> Self {
        Self { schema }
    }
}

struct Field<'de> {
    name: &'de str,
    schema: &'de Schema,
}

struct FieldSelection<'de> {
    fields: &'de FieldMap,
}

struct FieldVisitor<'de> {
    fields: &'de FieldMap,
}

impl FieldVisitor<'_> {
    pub fn new<'de>(fields: &'de FieldMap) -> FieldVisitor<'de> {
        FieldVisitor { fields }
    }
}

impl<'de> de::Visitor<'de> for FieldVisitor<'de> {
    type Value = Field<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a field name")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match self.fields.iter().find(|(u, _)| u == v) {
            Some((name, schema)) => Ok(Field { name, schema }),
            None => Err(de::Error::unknown_field(v, &[])),
        }
    }
}

impl FieldSelection<'_> {
    pub fn new<'de>(fields: &'de FieldMap) -> FieldSelection<'de> {
        FieldSelection { fields }
    }
}

impl<'de> de::DeserializeSeed<'de> for FieldSelection<'de> {
    type Value = Field<'de>;
    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let visitor = FieldVisitor::new(self.fields);
        deserializer.deserialize_identifier(visitor)
    }
}

impl<'de> de::DeserializeSeed<'de> for Deserialize<'de> {
    type Value = Value;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let visitor = Visitor::new(self.schema);
        match &self.schema {
            Schema::Primitive(schema) => match schema {
                schema::Primitive::Boolean => deserializer.deserialize_bool(visitor),
                schema::Primitive::Number(n) => match n {
                    schema::N::I64 => deserializer.deserialize_i64(visitor),
                    schema::N::U64 => deserializer.deserialize_u64(visitor),
                    schema::N::F64 => deserializer.deserialize_f64(visitor),
                },
                schema::Primitive::String => deserializer.deserialize_str(visitor),
            },
            Schema::Object(_) => deserializer.deserialize_map(visitor),
            Schema::Table { row: _, head: _ } => deserializer.deserialize_seq(visitor),
            Schema::Array(_) => deserializer.deserialize_seq(visitor),
        }
    }
}

struct Visitor<'de> {
    schema: &'de Schema,
}

impl<'de> Visitor<'de> {
    pub fn new(schema: &'de Schema) -> Self {
        Self { schema }
    }
}

struct RowVisitor<'de> {
    schema: &'de [Schema],
}

struct Row {
    cols: Vec<Value>,
}

impl<'de> RowVisitor<'de> {
    pub fn new(schema: &'de [Schema]) -> Self {
        Self { schema }
    }
}

impl<'de> de::DeserializeSeed<'de> for RowVisitor<'de> {
    type Value = Row;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> de::Visitor<'de> for RowVisitor<'de> {
    type Value = Row;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a row")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut cols = Vec::new();

        for schema in self.schema {
            let value = seq.next_element_seed(Deserialize::new(schema));

            match value {
                Ok(Some(value)) => cols.push(value),
                Ok(None) => return Err(de::Error::invalid_length(cols.len(), &"expected more")),
                Err(err) => return Err(err),
            }
        }
        Ok(Row { cols })
    }
}

struct Primitive<'de> {
    schema: &'de schema::Primitive,
}

impl<'de> Primitive<'de> {
    fn new(schema: &'de schema::Primitive) -> Self {
        Self { schema }
    }
}

impl<'de> de::Visitor<'de> for Primitive<'de> {
    type Value = value::Primitive;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.schema {
            schema::Primitive::String => formatter.write_str("a string"),
            schema::Primitive::Boolean => formatter.write_str("a boolean"),
            schema::Primitive::Number(n) => match n {
                schema::N::I64 => formatter.write_str("a i64"),
                schema::N::U64 => formatter.write_str("a u64"),
                schema::N::F64 => formatter.write_str("a f64"),
            },
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(value::Primitive::from_string(value.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value::Primitive::from_string(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value::Primitive::from_bool(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value::Primitive::from_f64(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value::Primitive::from_u64(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value::Primitive::from_i64(v))
    }
}

impl<'de> de::DeserializeSeed<'de> for Primitive<'de> {
    type Value = value::Primitive;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match self.schema {
            schema::Primitive::String => deserializer.deserialize_str(self),
            schema::Primitive::Boolean => deserializer.deserialize_bool(self),
            schema::Primitive::Number(n) => match n {
                schema::N::I64 => deserializer.deserialize_i64(self),
                schema::N::U64 => deserializer.deserialize_u64(self),
                schema::N::F64 => deserializer.deserialize_f64(self),
            },
        }
    }
}

impl<'de> serde::de::Visitor<'de> for Visitor<'de> {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.schema {
            Schema::Primitive(schema) => match schema {
                schema::Primitive::String => formatter.write_str("a string"),
                schema::Primitive::Boolean => formatter.write_str("a boolean"),
                schema::Primitive::Number(n) => match n {
                    schema::N::I64 => formatter.write_str("a i64"),
                    schema::N::U64 => formatter.write_str("a u64"),
                    schema::N::F64 => formatter.write_str("a f64"),
                },
            },
            Schema::Object(_) => formatter.write_str("an object"),
            Schema::Table { row: _, head: _ } => formatter.write_str("a table"),
            Schema::Array(_) => formatter.write_str("an array"),
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::from_string(value.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from_string(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from_bool(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from_f64(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from_u64(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::from_i64(v))
    }

    #[inline]
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        if let Schema::Object(fields) = self.schema {
            let mut rows = Vec::new();
            while let Ok(Some(field)) = map.next_key_seed(FieldSelection::new(fields.as_slice())) {
                let value_schema = field.schema;
                match map.next_value_seed(Deserialize::new(&value_schema)) {
                    Ok(value) => rows.push((field.name.to_owned(), value)),
                    Err(err) => return Err(err),
                };
            }

            Ok(Value::Object(rows))
        } else {
            Err(de::Error::custom("expected object"))
        }
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        match self.schema {
            Schema::Table { head, row } => {
                let mut rows = Vec::with_capacity(seq.size_hint().unwrap_or(100));
                while let Ok(Some(row)) = seq.next_element_seed(RowVisitor::new(row)) {
                    rows.push(row.cols);
                }

                Ok(Value::Table { head: head.to_owned(), rows })
            }
            Schema::Array(primitive) => {
                let mut rows = Vec::with_capacity(seq.size_hint().unwrap_or(100));
                while let Ok(Some(row)) = seq.next_element_seed(Primitive::new(primitive)) {
                    rows.push(row);
                }

                Ok(Value::Array(rows))
            }
            _ => Err(de::Error::custom("expected a table or an array")),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::schema::{Schema, N};
    use insta::assert_snapshot;

    #[test]
    fn test_string() {
        let schema = Schema::string();
        let input = r#""Hello World!""#;
        let actual = schema.from_str(input).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    fn test_bool() {
        let schema = Schema::boolean();
        let input = r#"true"#;
        let actual = schema.from_str(input).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    fn test_i32() {
        let schema = Schema::u64();
        let actual = schema.from_str(r#"42"#).unwrap();
        assert_snapshot!(actual);

        let actual = schema.from_str(r#"-42"#).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    fn test_u64() {
        let schema = Schema::u64();
        let input = r#"42"#;
        let actual = schema.from_str(input).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    fn test_f64() {
        let schema = Schema::f64();
        let input = r#"42.0"#;
        let actual = schema.from_str(input).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    fn test_object() {
        let schema = Schema::object(vec![(("foo", Schema::u64())), (("bar", Schema::boolean()))]);
        let input = r#"{"foo": 42, "bar": true}"#;
        let actual = schema.from_str(input).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    fn test_array() {
        let schema = Schema::array(schema::Primitive::u64());
        let input = r#"[1, 2, 3]"#;
        let actual = schema.from_str(input).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    fn test_table() {
        let schema = Schema::table(&["foo", "bar"], &[Schema::u64(), Schema::string()]);
        let input = r#"[{"foo": 1, bar: "Hello"}, {"foo": 2, bar: "Bye"}]"#;
        let actual = schema.from_str(input).unwrap();
        assert_snapshot!(actual);
    }

    #[test]
    #[ignore]
    fn test_posts() {
        const JSON: &str = include_str!("../data/posts.json");
        let schema = Schema::table(
            &["user_id", "id", "title", "body"],
            &[
                Schema::u64(),
                Schema::u64(),
                Schema::string(),
                Schema::string(),
            ],
        );
        let actual = schema.from_str(JSON).unwrap();
        assert_snapshot!(actual);
    }
}
