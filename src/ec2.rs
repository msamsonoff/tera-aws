use std::collections::HashMap;
use std::sync::Arc;

use aws_sdk_ec2::model::{Filter, ResourceType};
use aws_sdk_ec2::Client;
use aws_types::config::Config;
use eyre::Result;
use serde::Deserialize;
use serde_json::{from_value, Map, Value};
use tera::{Function, Tera};
use tokio::runtime::Runtime;

pub fn register(tera: &mut Tera, runtime: &Arc<Runtime>, config: &Config) {
    let client = Client::new(config);
    let ec2_describe_tags = Ec2DescribeTags::new(runtime, client);
    tera.register_function("ec2_describe_tags", ec2_describe_tags);
}

pub struct Ec2DescribeTags {
    runtime: Arc<Runtime>,
    client: Client,
}

impl Ec2DescribeTags {
    pub fn new(runtime: &Arc<Runtime>, client: Client) -> Self {
        Ec2DescribeTags {
            runtime: runtime.clone(),
            client,
        }
    }
}

impl Function for Ec2DescribeTags {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let filters = match args.get("filters") {
            None => None,
            Some(filters) => {
                let filters = filters.clone();
                let filters: Vec<String> = from_value(filters)
                    .map_err(|err| tera::Error::msg(format!("Function `ec2_describe_tags` received filters={} but `filters` can only be an array of strings", err)))?;
                let filters: Vec<Filter> = filters
                    .into_iter()
                    .map(try_parse_filter)
                    .collect::<tera::Result<Vec<Filter>>>()?;
                Some(filters)
            }
        };
        let future = self.client.describe_tags().set_filters(filters).send();
        let response = self
            .runtime
            .block_on(future)
            .map_err(|err| tera::Error::msg(format!("{}", err)))?;
        let value = match response.tags() {
            None => Value::Null,
            Some(tag_descriptions) => {
                let tag_descriptions: Vec<_> = tag_descriptions
                    .iter()
                    .map(|tag_description| {
                        let mut map = Map::new();
                        tag_description
                            .key()
                            .insert_nullable_string(&mut map, "key");
                        tag_description
                            .resource_id()
                            .insert_nullable_string(&mut map, "resource_id");
                        tag_description
                            .resource_type()
                            .insert_nullable_string(&mut map, "resource_type");
                        tag_description
                            .value()
                            .insert_nullable_string(&mut map, "value");
                        Value::Object(map)
                    })
                    .collect();
                Value::Array(tag_descriptions)
            }
        };
        Ok(value)
    }
}

fn try_parse_filter<S>(s: S) -> tera::Result<Filter>
where
    S: AsRef<str>,
{
    let s = s.as_ref();
    try_parse_filter_as_option(s).ok_or_else(|| {
        tera::Error::msg(format!(
            "Function `ec2_describe_tags` received invalid filters={}",
            s
        ))
    })
}

fn try_parse_filter_as_option(s: &str) -> Option<Filter> {
    match s.strip_prefix("Name=")?.split_once(',') {
        None => {
            let filter = Filter::builder().name(s).build();
            Some(filter)
        }
        Some((name, values)) => {
            let values: Vec<_> = values
                .strip_prefix("Values=")?
                .split(',')
                .map(String::from)
                .collect();
            let filter = Filter::builder()
                .name(name)
                .set_values(Some(values))
                .build();
            Some(filter)
        }
    }
}

trait TagDescriptionFieldExt {
    fn insert_nullable_string<K>(&self, map: &mut Map<String, Value>, key: K)
    where
        K: AsRef<str>;
}

impl TagDescriptionFieldExt for Option<&str> {
    fn insert_nullable_string<K>(&self, map: &mut Map<String, Value>, key: K)
    where
        K: AsRef<str>,
    {
        let key = key.as_ref().to_string();
        let value = self
            .map(|s| Value::String(s.to_string()))
            .unwrap_or(Value::Null);
        map.insert(key, value);
    }
}

impl TagDescriptionFieldExt for Option<&ResourceType> {
    fn insert_nullable_string<K>(&self, map: &mut Map<String, Value>, key: K)
    where
        K: AsRef<str>,
    {
        let key = key.as_ref().to_string();
        let value = self
            .map(|s| Value::String(s.as_str().to_string()))
            .unwrap_or(Value::Null);
        map.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_parse_filter() {
        let left = try_parse_filter("Name=X,Values=Y,Z").unwrap();
        let right = Filter::builder().name("X").values("Y").values("Z").build();
        assert_eq!(left, right);
    }

    #[test]
    fn test_try_parse_filter_name_only() {
        let left = try_parse_filter("Name=X").unwrap();
        let right = Filter::builder().name("X").build();
        assert_eq!(left, right);
    }

    #[test]
    #[should_panic]
    fn test_try_parse_filter_values_only() {
        let left = try_parse_filter("Values=Y,Z").unwrap();
    }

    #[test]
    #[should_panic]
    fn test_try_prase_filter_empty() {
        let left = try_parse_filter("").unwrap();
        let right = Filter::builder().name("X").build();
        assert_eq!(left, right);
    }
}
