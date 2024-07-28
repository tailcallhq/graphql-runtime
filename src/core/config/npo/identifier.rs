use std::collections::{HashMap, HashSet};
use std::fmt::Display;

use super::Queries;
use crate::core::config::Config;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct TypeName<'a>(pub &'a str);
impl<'a> TypeName<'a> {
    pub fn new(name: &'a str) -> Self {
        Self(name)
    }
    pub fn as_str(self) -> &'a str {
        self.0
    }
}
impl Display for TypeName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FieldName<'a>(&'a str);
impl<'a> FieldName<'a> {
    pub fn new(name: &'a str) -> Self {
        Self(name)
    }
    pub fn as_str(self) -> &'a str {
        self.0
    }
}
impl Display for FieldName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Identifier<'a> {
    config: &'a Config,
    visited: HashMap<TypeName<'a>, HashSet<FieldName<'a>>>,
}

impl<'a> Identifier<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config, visited: HashMap::new() }
    }

    pub fn identify(mut self) -> Queries<'a> {
        if let Some(query) = &self.config.schema.query {
            self.find_fan_out(query, false)
        } else {
            Default::default()
        }
    }
    #[inline(always)]
    fn find_fan_out(&mut self, type_name: &'a str, is_list: bool) -> Queries<'a> {
        let config = self.config;
        let type_name: TypeName = TypeName(type_name);
        let mut ans = HashMap::new();

        if let Some(type_) = config.find_type(type_name.0) {
            for (field_name, field) in type_.fields.iter() {
                let cur: FieldName = FieldName(field_name.as_str());
                let tuple: (FieldName, TypeName) = (cur, TypeName(field.type_of.as_str()));

                let condition = self
                    .visited
                    .get(&type_name)
                    .map(|v: &HashSet<FieldName>| v.contains(&cur))
                    .unwrap_or_default();

                if condition {
                    continue;
                } else {
                    self.visited.entry(type_name).or_default().insert(cur);
                }

                if field.has_resolver() && !field.has_batched_resolver() && is_list {
                    ans.entry(type_name).or_insert(HashSet::new()).insert(tuple);
                } else {
                    let next = self.find_fan_out(&field.type_of, field.list || is_list);
                    for (k, v) in next.map() {
                        ans.entry(*k).or_insert(HashSet::new()).extend(v);
                        ans.entry(type_name).or_insert(HashSet::new()).insert(tuple);
                    }
                }
            }
        }

        Queries::new(ans, type_name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::core::config::npo::Queries;
    use crate::core::config::{Config, Field, Http, Type};

    macro_rules! assert_eq_map {
        ($actual:expr, $expected_vec:expr) => {{
            let mut expected: HashMap<TypeName, HashSet<(FieldName, TypeName)>> = HashMap::new();

            for vec in $expected_vec {
                for value in vec {
                    let (key, (value, ty_of)) = value;
                    let key = TypeName(key);
                    let value = (FieldName(value), TypeName(ty_of));

                    expected
                        .entry(key)
                        .or_insert_with(HashSet::new)
                        .insert(value);
                }
            }

            assert_eq!($actual, Queries::new(expected, ($actual).root()));
        }};
    }

    #[test]
    fn test_nplusone_resolvers() {
        let config = Config::default().query("Query").types(vec![
            (
                "Query",
                Type::default().fields(vec![(
                    "f1",
                    Field::default()
                        .type_of("F1".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "F1",
                Type::default().fields(vec![(
                    "f2",
                    Field::default()
                        .type_of("F2".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "F2",
                Type::default()
                    .fields(vec![("f3", Field::default().type_of("String".to_string()))]),
            ),
        ]);
        let actual = config.n_plus_one();
        let expected = vec![vec![("Query", ("f1", "F1")), ("F1", ("f2", "F2"))]];

        assert_eq_map!(actual, expected);
    }

    #[test]
    fn test_nplusone_batched_resolvers() {
        let config = Config::default().query("Query").types(vec![
            (
                "Query",
                Type::default().fields(vec![(
                    "f1",
                    Field::default()
                        .type_of("F1".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "F1",
                Type::default().fields(vec![(
                    "f2",
                    Field::default()
                        .type_of("F2".to_string())
                        .into_list()
                        .http(Http { batch_key: vec!["id".into()], ..Default::default() }),
                )]),
            ),
            (
                "F2",
                Type::default()
                    .fields(vec![("f3", Field::default().type_of("String".to_string()))]),
            ),
        ]);

        let actual = config.n_plus_one();
        let expected: Vec<Vec<_>> = vec![];
        assert_eq_map!(actual, expected);
    }

    #[test]
    fn test_nplusone_nested_resolvers() {
        let config = Config::default().query("Query").types(vec![
            (
                "Query",
                Type::default().fields(vec![(
                    "f1",
                    Field::default()
                        .type_of("F1".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "F1",
                Type::default().fields(vec![(
                    "f2",
                    Field::default().type_of("F2".to_string()).into_list(),
                )]),
            ),
            (
                "F2",
                Type::default().fields(vec![(
                    "f3",
                    Field::default().type_of("F3".to_string()).into_list(),
                )]),
            ),
            (
                "F3",
                Type::default().fields(vec![(
                    "f4",
                    Field::default()
                        .type_of("String".to_string())
                        .http(Http::default()),
                )]),
            ),
        ]);

        let actual = config.n_plus_one();
        let expected = vec![vec![
            ("Query", ("f1", "F1")),
            ("F1", ("f2", "F2")),
            ("F2", ("f3", "F3")),
            ("F3", ("f4", "String")),
        ]];

        assert_eq_map!(actual, expected);
    }

    #[test]
    fn test_nplusone_nested_resolvers_non_list_resolvers() {
        let config = Config::default().query("Query").types(vec![
            (
                "Query",
                Type::default().fields(vec![(
                    "f1",
                    Field::default()
                        .type_of("F1".to_string())
                        .http(Http::default()),
                )]),
            ),
            (
                "F1",
                Type::default().fields(vec![(
                    "f2",
                    Field::default().type_of("F2".to_string()).into_list(),
                )]),
            ),
            (
                "F2",
                Type::default().fields(vec![(
                    "f3",
                    Field::default().type_of("F3".to_string()).into_list(),
                )]),
            ),
            (
                "F3",
                Type::default().fields(vec![(
                    "f4",
                    Field::default()
                        .type_of("String".to_string())
                        .http(Http::default()),
                )]),
            ),
        ]);

        let expected = vec![vec![
            ("Query", ("f1", "F1")),
            ("F1", ("f2", "F2")),
            ("F2", ("f3", "F3")),
            ("F3", ("f4", "String")),
        ]];
        let actual = config.n_plus_one();

        assert_eq_map!(actual, expected);
    }

    #[test]
    fn test_nplusone_nested_resolvers_without_resolvers() {
        let config = Config::default().query("Query").types(vec![
            (
                "Query",
                Type::default().fields(vec![(
                    "f1",
                    Field::default()
                        .type_of("F1".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "F1",
                Type::default().fields(vec![(
                    "f2",
                    Field::default().type_of("F2".to_string()).into_list(),
                )]),
            ),
            (
                "F2",
                Type::default()
                    .fields(vec![("f3", Field::default().type_of("String".to_string()))]),
            ),
        ]);

        let expected: Vec<Vec<_>> = vec![];
        let actual = config.n_plus_one();

        assert_eq_map!(actual, expected);
    }

    #[test]
    fn test_nplusone_cycles() {
        let config = Config::default().query("Query").types(vec![
            (
                "Query",
                Type::default().fields(vec![(
                    "f1",
                    Field::default()
                        .type_of("F1".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "F1",
                Type::default().fields(vec![
                    ("f1", Field::default().type_of("F1".to_string())),
                    ("f2", Field::default().type_of("F2".to_string()).into_list()),
                ]),
            ),
            (
                "F2",
                Type::default()
                    .fields(vec![("f3", Field::default().type_of("String".to_string()))]),
            ),
        ]);

        let actual = config.n_plus_one();
        let expected: Vec<Vec<_>> = vec![];

        assert_eq_map!(actual, expected);
    }

    #[test]
    fn test_nplusone_cycles_with_resolvers() {
        let config = Config::default().query("Query").types(vec![
            (
                "Query",
                Type::default().fields(vec![(
                    "f1",
                    Field::default()
                        .type_of("F1".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "F1",
                Type::default().fields(vec![
                    ("f1", Field::default().type_of("F1".to_string()).into_list()),
                    (
                        "f2",
                        Field::default()
                            .type_of("String".to_string())
                            .http(Http::default()),
                    ),
                ]),
            ),
            (
                "F2",
                Type::default()
                    .fields(vec![("f3", Field::default().type_of("String".to_string()))]),
            ),
        ]);

        let actual = config.n_plus_one();
        let expected = vec![
            vec![
                ("Query", ("f1", "F1")),
                ("F1", ("f1", "F1")),
                ("F1", ("f2", "String")),
            ],
            vec![("Query", ("f1", "F1")), ("F1", ("f2", "String"))],
        ];

        assert_eq_map!(actual, expected);
    }
    #[test]
    fn test_nplusone_nested_non_list() {
        let f_field = Field::default()
            .type_of("F".to_string())
            .http(Http::default());

        let config = Config::default().query("Query").types(vec![
            ("Query", Type::default().fields(vec![("f", f_field)])),
            (
                "F",
                Type::default().fields(vec![(
                    "g",
                    Field::default()
                        .type_of("G".to_string())
                        .into_list()
                        .http(Http::default()),
                )]),
            ),
            (
                "G",
                Type::default().fields(vec![("e", Field::default().type_of("String".to_string()))]),
            ),
        ]);

        let actual = config.n_plus_one();
        let expected = Vec::<Vec<_>>::new();

        assert_eq_map!(actual, expected);
    }
}
