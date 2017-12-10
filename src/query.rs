/// The Toml Query extensions

use toml::Value;

use read::TomlValueReadExt;
use error::Result;
use error::ErrorKind as EK;

/// A Query which can be executed on a `Value` object
///
/// The `Queryable::execute_query()` method can be used to execute a query on a `toml::Value`
/// document. The target passed to the `Query::execute` function is the object which is yielded by
/// the `query_path()` (if any).
pub trait Query {
    type Output: Sized;

    /// Execute a query on a target which is yielded by calling `Value::read_mut()` with the path
    /// yielded by `Query::query_path()`.
    fn execute<'doc>(&self, target: &mut Value) -> Result<Self::Output>;
    fn query_path(&self) -> &str;

    /// An optional query which shall be executed _after_ this query is executed.
    /// The default implementation yields None.
    fn next(&self) -> Option<&Query<Output = Self::Output>> {
        None
    }

}

pub trait Queryable<'doc> : TomlValueReadExt<'doc> {
    fn execute_query<Q: Query>(&'doc mut self, q: &Q) -> Result<Q::Output>;
}

impl<'doc> Queryable<'doc> for Value {

    /// Execute a query on the Value.
    ///
    /// # Return value
    ///
    /// If the passed Query does not yield a `next()`, the return value of the Query is
    /// returned.
    ///
    /// If the passed Query yields a `next()`, the return value of this query is dropped and
    /// the `q.next()` query is executed, with the `execute_query()` function itself, so the
    /// same semantics apply.
    ///
    fn execute_query<Q: Query>(&'doc mut self, q: &Q) -> Result<Q::Output> {
        match q.next() {
            Some(ref next) => {
                self.read_mut(q.query_path())?
                    .map(|v| {
                        let _ = q.execute(v);
                        next.execute(v)
                    })
                    .unwrap_or(Err(EK::NotAvailable(String::from(q.query_path())).into()))
            },

            None => {
                self.read_mut(q.query_path())?
                    .map(|v| q.execute(v))
                    .unwrap_or(Err(EK::NotAvailable(String::from(q.query_path())).into()))
            }
        }
    }
}

pub mod read {
    use super::Query;
    use error::Result;
    use toml::Value;

    pub struct ReadQuery<'a> {
        path: &'a str
    }

    impl<'a> ReadQuery<'a> {
        pub fn new(path: &'a str) -> ReadQuery {
            ReadQuery { path: path }
        }
    }

    impl<'a> Query for ReadQuery<'a> {
        type Output = Value;

        fn execute<'doc>(&self, target: &'doc mut Value) -> Result<Self::Output> {
            Ok(target.clone())
        }

        fn query_path(&self) -> &str {
            self.path
        }
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use toml::from_str as toml_from_str;

    struct TestQuery;

    impl Query for TestQuery {
        type Output = Vec<i64>;

        fn execute<'doc>(&self, target: &'doc mut Value) -> Result<Self::Output> {
            match *target {
                Value::Integer(i) => Ok(vec![i]),
                _ => panic!("Unexpected target"),
            }
        }

        fn query_path(&self) -> &str {
            "table.a"
        }
    }

    #[test]
    fn test_type_converting_query_1() {
        let mut toml : Value = toml_from_str(r#"
        [table]
        a = 1
        "#).unwrap();

        let query = TestQuery {};

        let val  = toml.execute_query(&query);
        assert!(val.is_ok());
        assert_eq!(val.unwrap(), vec![1]);
    }


    struct PersonQuery;
    struct Person {
        pub first_name: String,
        pub last_name: String,
        pub age: i64,
    }

    impl Query for PersonQuery {
        type Output = Person;

        fn execute<'doc>(&self, t: &'doc mut Value) -> Result<Self::Output> {
            use error::ErrorKind as EK;

            let first_name = t.read("first_name")?.ok_or(EK::NotAvailable("first_name".to_owned()))?;
            let last_name = t.read("last_name")?.ok_or(EK::NotAvailable("last_name".to_owned()))?;
            let age = t.read("age")?.ok_or(EK::NotAvailable("age".to_owned()))?;

            let first_name = match first_name {
                &Value::String(ref s) => s.clone(),
                _ => panic!("Wrong type"),
            };
            let last_name = match last_name {
                &Value::String(ref s) => s.clone(),
                _ => panic!("Wrong type"),
            };
            let age = match *age {
                Value::Integer(i) => i,
                _ => panic!("Wrong type"),
            };

            Ok(Person {
                first_name: first_name,
                last_name: last_name,
                age: age
            })
        }

        fn query_path(&self) -> &str {
            "table.a"
        }
    }

    #[test]
    fn test_type_converting_query_2() {
        let mut toml : Value = toml_from_str(r#"
        [table.a]
        first_name = "Max"
        last_name = "Mustermann"
        age = 77
        "#).unwrap();

        let query = PersonQuery{};

        let val  = toml.execute_query(&query);

        assert!(val.is_ok());
        let val = val.unwrap();

        assert_eq!(val.first_name, "Max");
        assert_eq!(val.last_name, "Mustermann");
        assert_eq!(val.age, 77);
    }
}

