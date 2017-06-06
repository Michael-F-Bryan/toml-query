/// The query resolver that operates on the AST and the TOML object

use std::collections::BTreeMap;

use toml::Value;
use tokenizer::Token;
use error::*;

pub fn resolve<'doc>(toml: &'doc mut Value, tokens: &Token) -> Result<&'doc mut Value> {
    match toml {
        &mut Value::Table(ref mut t) => {
            match tokens {
                &Token::Identifier { ref ident, .. } => {
                    let mut sub_document = t
                        .entry(ident.clone())
                        .or_insert(Value::Table(BTreeMap::new()));

                    match tokens.next() {
                        Some(next) => resolve(sub_document, next),
                        None => Ok(sub_document),
                    }
                },

                &Token::Index { idx, .. } => {
                    let kind = ErrorKind::NoIndexInTable(idx);
                    Err(Error::from(kind))
                },
            }
        },

        &mut Value::Array(ref mut ary) => {
            match tokens {
                &Token::Index { idx, .. } => {
                    match tokens.next() {
                        Some(next) => resolve(ary.get_mut(idx).unwrap(), next),
                        None       => Ok(ary.index_mut(idx)),
                    }
                },
                &Token::Identifier { ref ident, .. } => {
                    let kind = ErrorKind::NoIdentifierInArray(ident.clone());
                    Err(Error::from(kind))
                },
            }
        },

        _ => match tokens {
            &Token::Identifier { ref ident, .. } => {
                Err(Error::from(ErrorKind::QueryingValueAsTable(ident.clone())))
            },

            &Token::Index { idx, .. } => {
                Err(Error::from(ErrorKind::QueryingValueAsArray(idx)))
            },
        }
    }
}

#[cfg(test)]
mod test {
    use toml::from_str as toml_from_str;
    use toml::Value;
    use tokenizer::*;
    use super::resolve;

    macro_rules! do_resolve {
        ( $toml:ident => $query:expr ) => {
            resolve(&mut $toml, &tokenize_with_seperator(&String::from($query), '.').unwrap())
        }
    }

    #[test]
    fn test_resolve_empty_toml_simple_query() {
        let mut toml = toml_from_str("").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(is_match!(result, &mut Value::Table(_)));
        match result {
            &mut Value::Table(ref tab) => assert!(tab.is_empty()),
            _ => assert!(false, "Expected Table, got something else"),
        }
    }

    #[test]
    fn test_resolve_present_bool() {
        let mut toml = toml_from_str("example = true").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Boolean(true)));
    }

    #[test]
    fn test_resolve_present_integer() {
        let mut toml = toml_from_str("example = 1").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Integer(1)));
    }

    #[test]
    fn test_resolve_present_float() {
        let mut toml = toml_from_str("example = 1.0").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Float(1.0)));
    }

    #[test]
    fn test_resolve_present_string() {
        let mut toml = toml_from_str("example = 'string'").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::String(_)));
        match result {
            &mut Value::String(ref s) => assert_eq!("string", s),
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_present_array_bools() {
        let mut toml = toml_from_str("example = [ true, false ]").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Array(_)));
        match result {
            &mut Value::Array(ref ary) => {
                assert_eq!(ary[0], Value::Boolean(true));
                assert_eq!(ary[1], Value::Boolean(false));
            },
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_present_array_integers() {
        let mut toml = toml_from_str("example = [ 1, 1337 ]").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Array(_)));
        match result {
            &mut Value::Array(ref ary) => {
                assert_eq!(ary[0], Value::Integer(1));
                assert_eq!(ary[1], Value::Integer(1337));
            },
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_present_array_floats() {
        let mut toml = toml_from_str("example = [ 1.0, 133.25 ]").unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Array(_)));
        match result {
            &mut Value::Array(ref ary) => {
                assert_eq!(ary[0], Value::Float(1.0));
                assert_eq!(ary[1], Value::Float(133.25));
            },
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_array_index_query_1() {
        let mut toml = toml_from_str("example = [ 1 ]").unwrap();
        let result = do_resolve!(toml => "example.[0]");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Integer(1)));
    }

    #[test]
    fn test_resolve_array_index_query_2() {
        let mut toml = toml_from_str("example = [ 1, 2, 3, 4, 5 ]").unwrap();
        let result = do_resolve!(toml => "example.[4]");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Integer(5)));
    }

    #[test]
    fn test_resolve_table_element_query() {
        let mut toml = toml_from_str(r#"
        [table]
        value = 42
        "#).unwrap();
        let result = do_resolve!(toml => "table.value");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Integer(42)));
    }

    #[test]
    fn test_resolve_table_with_many_elements_element_query() {
        let mut toml = toml_from_str(r#"
        [table]
        value1 = 42
        value2 = 43
        value3 = 44
        value4 = 45
        value5 = 46
        "#).unwrap();
        let result = do_resolve!(toml => "table.value1");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Integer(42)));
    }

    #[test]
    fn test_resolve_table_array_query() {
        let mut toml = toml_from_str(r#"
        [table]
        value1 = [ 42.0, 50.0 ]
        "#).unwrap();
        let result = do_resolve!(toml => "table.value1");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Array(_)));
        match result {
            &mut Value::Array(ref ary) => {
                assert_eq!(ary[0], Value::Float(42.0));
                assert_eq!(ary[1], Value::Float(50.0));
            },
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_table_array_element_query() {
        let mut toml = toml_from_str(r#"
        [table]
        value1 = [ 42 ]
        "#).unwrap();
        let result = do_resolve!(toml => "table.value1.[0]");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Integer(42)));
    }

    #[test]
    fn test_resolve_multi_table_query() {
        let mut toml = toml_from_str(r#"
        [table0]
        value = [ 1 ]
        [table1]
        value = [ "Foo" ]
        [table2]
        value = [ 42.0 ]
        [table3]
        value = [ true ]
        "#).unwrap();
        let result = do_resolve!(toml => "table1.value.[0]");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::String(_)));
        match result {
            &mut Value::String(ref s) => assert_eq!("Foo", s),
            _ => panic!("What just happened?"),
        }
    }

    static FRUIT_TABLE : &'static str = r#"
    [[fruit.blah]]
      name = "apple"

      [fruit.blah.physical]
        color = "red"
        shape = "round"

    [[fruit.blah]]
      name = "banana"

      [fruit.blah.physical]
        color = "yellow"
        shape = "bent"
    "#;

    #[test]
    fn test_resolve_array_table_query_1() {
        let mut toml = toml_from_str(FRUIT_TABLE).unwrap();
        let result = do_resolve!(toml => "fruit.blah.[0].name");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::String(_)));
        match result {
            &mut Value::String(ref s) => assert_eq!("apple", s),
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_array_table_query_2() {
        let mut toml = toml_from_str(FRUIT_TABLE).unwrap();
        let result = do_resolve!(toml => "fruit.blah.[0].physical");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Table(_)));
        match result {
            &mut Value::Table(ref tab) => {
                match tab.get("color") {
                    Some(&Value::String(ref s)) => assert_eq!("red", s),
                    _ => assert!(false),
                }
                match tab.get("shape") {
                    Some(&Value::String(ref s)) => assert_eq!("round", s),
                    _ => assert!(false),
                }
            },
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_query_on_result() {
        let mut toml = toml_from_str(FRUIT_TABLE).unwrap();
        let result = do_resolve!(toml => "fruit.blah.[1].physical");

        assert!(result.is_ok());
        let mut result = result.unwrap();

        let tokens = tokenize_with_seperator(&String::from("color"), '.').unwrap();
        let result = resolve(result, &tokens);

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::String(_)));
        match result {
            &mut Value::String(ref s) => assert_eq!("yellow", s),
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_query_empty_table() {
        let mut toml = toml_from_str(r#"
        [example]
        "#).unwrap();
        let result = do_resolve!(toml => "example");

        assert!(result.is_ok());
        let result = result.unwrap();

        assert!(is_match!(result, &mut Value::Table(_)));
        match result {
            &mut Value::Table(ref t) => assert!(t.is_empty()),
            _ => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_query_member_of_empty_table() {
        let mut toml = toml_from_str("").unwrap();
        let result = do_resolve!(toml => "example.foo");

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(is_match!(result, &mut Value::Table(_)));
        match result {
            &mut Value::Table(ref t) => assert!(t.is_empty()),
            _                        => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_query_index_in_table() {
        let mut toml = toml_from_str("").unwrap();
        let result = do_resolve!(toml => "example.[0]");

        assert!(result.is_err());
        let result = result.unwrap_err();

        let errkind = result.kind();
        assert!(is_match!(errkind, &ErrorKind::NoIndexInTable { .. }));
    }

    #[test]
    fn test_resolve_query_identifier_in_array() {
        let mut toml = toml_from_str("").unwrap();
        let result = do_resolve!(toml => "example.foo.bar");

        assert!(result.is_ok());
        let result = result.unwrap();

        match result {
            &mut Value::Table(ref t) => assert!(t.is_empty()),
            _                        => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_query_value_as_table() {
        let mut toml = toml_from_str("").unwrap();
        let result = do_resolve!(toml => "example.foo.bar");

        assert!(result.is_ok());
        let result = result.unwrap();

        match result {
            &mut Value::Table(ref t) => assert!(t.is_empty()),
            _                        => panic!("What just happened?"),
        }
    }

    #[test]
    fn test_resolve_query_value_as_array() {
        let mut toml = toml_from_str("").unwrap();
        let result = do_resolve!(toml => "example.foo.[0]");

        assert!(result.is_ok());
        let result = result.unwrap();

        match result {
            &mut Value::Array(ref a) => assert!(a.is_empty()),
            _                        => panic!("What just happened?"),
        }
    }

}

