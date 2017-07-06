use std::str::FromStr;
use std::collections::HashMap;

use quire::validate::{Structure, Scalar, Numeric, Mapping};
use quire::{Options, parse_string};

#[derive(RustcDecodable, RustcEncodable, Serialize, Deserialize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ChildKind {
    Daemon,
    Command,
}

#[derive(RustcDecodable, Serialize, Deserialize, PartialEq, Debug)]
pub struct ChildConfig {
    pub instances: usize,
    pub image: String,
    pub config: String,
    #[serde(skip_serializing_if="HashMap::is_empty", default)]
    pub variables: HashMap<String, String>,
    pub kind: ChildKind,
}

impl ChildConfig {
    pub fn mapping_validator<'x>() -> Mapping<'x> {
        return Mapping::new(
            Scalar::new(),
            ChildConfig::validator());
    }
    pub fn validator<'x>() -> Structure<'x> {
        Structure::new()
        .member("instances", Numeric::new().default(1))
        .member("image", Scalar::new())
        .member("config", Scalar::new())
        .member("variables", Mapping::new(Scalar::new(), Scalar::new()))
        .member("kind", Scalar::new().default("Daemon"))
    }
}

impl FromStr for ChildConfig {
    type Err = ();
    fn from_str(body: &str) -> Result<ChildConfig, ()> {
        parse_string("<command-line>", body,
            &ChildConfig::validator(), &Options::default())
            .map_err(|_| ())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::str::FromStr;
    use super::ChildConfig;
    use super::ChildKind::Daemon;
    use serde_json::{to_string, from_str};

    #[test]
    fn deserialize_compat() {
        let data = r#"{
            "instances":1,
            "image":"myproj.4a20772b",
            "config":"/config/staging/myproj.yaml",
            "kind":"Daemon"}"#;
        let cc = ChildConfig::from_str(data).unwrap();
        assert_eq!(cc, ChildConfig {
            instances: 1,
            image: String::from("myproj.4a20772b"),
            config: String::from("/config/staging/myproj.yaml"),
            variables: HashMap::new(),
            kind: Daemon,
        });

        let cc: ChildConfig = from_str(&data).unwrap();
        assert_eq!(cc, ChildConfig {
            instances: 1,
            image: String::from("myproj.4a20772b"),
            config: String::from("/config/staging/myproj.yaml"),
            variables: HashMap::new(),
            kind: Daemon,
        });
    }

    #[test]
    fn dedeserialize_vars() {
        let data = r#"{
            "instances":1,
            "image":"myproj.4a20772b",
            "config":"/config/staging/myproj.yaml",
            "variables": {"a": "b"},
            "kind":"Daemon"}"#;
        let cc = ChildConfig::from_str(data).unwrap();
        assert_eq!(cc, ChildConfig {
            instances: 1,
            image: String::from("myproj.4a20772b"),
            config: String::from("/config/staging/myproj.yaml"),
            variables: vec![
                (String::from("a"), String::from("b")),
            ].into_iter().collect(),
            kind: Daemon,
        })
    }

    #[test]
    fn serialize_compat() {
        let data = to_string(&ChildConfig {
            instances: 1,
            image: String::from("myproj.4a20772b"),
            config: String::from("/config/staging/myproj.yaml"),
            variables: HashMap::new(),
            kind: Daemon,
        }).unwrap();
        assert_eq!(data, "{\
            \"instances\":1,\
            \"image\":\"myproj.4a20772b\",\
            \"config\":\"/config/staging/myproj.yaml\",\
            \"kind\":\"Daemon\"}");
    }

    #[test]
    fn serialize_vars() {
        let data = to_string(&ChildConfig {
            instances: 1,
            image: String::from("myproj.4a20772b"),
            config: String::from("/config/staging/myproj.yaml"),
            variables: vec![
                (String::from("a"), String::from("b")),
            ].into_iter().collect(),
            kind: Daemon,
        }).unwrap();
        assert_eq!(data, "{\
            \"instances\":1,\
            \"image\":\"myproj.4a20772b\",\
            \"config\":\"/config/staging/myproj.yaml\",\
            \"variables\":{\"a\":\"b\"},\
            \"kind\":\"Daemon\"}");
    }
}
