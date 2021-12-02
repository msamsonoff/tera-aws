use std::collections::HashMap;
use std::sync::Arc;

use aws_sdk_secretsmanager::Client;
use aws_types::config::Config;
use serde_json::value::from_value;
use serde_json::{from_str, Value};
use tera::{Function, Tera};
use tokio::runtime::Runtime;

pub fn register(tera: &mut Tera, runtime: &Arc<Runtime>, config: &Config) {
    let client = Client::new(config);
    let client = Arc::new(client);
    let get_secret_value = GetSecretValue::new(runtime, &client);
    tera.register_function("secretsmanager_get_secret_value", get_secret_value);
    let get_secret_value_json = GetSecretValueJson::new(runtime, &client);
    tera.register_function(
        "secretsmanager_get_secret_value_json",
        get_secret_value_json,
    );
}

pub struct GetSecretValue {
    runtime: Arc<Runtime>,
    client: Arc<Client>,
}

impl GetSecretValue {
    pub fn new(runtime: &Arc<Runtime>, client: &Arc<Client>) -> Self {
        GetSecretValue {
            runtime: runtime.clone(),
            client: client.clone(),
        }
    }
}

impl Function for GetSecretValue {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        map_secret_string(
            "secretsmanager_get_secret_value",
            &self.runtime,
            &self.client,
            args,
            |_, secret_string| {
                let value = secret_string
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null);
                Ok(value)
            },
        )
    }
}

pub struct GetSecretValueJson {
    runtime: Arc<Runtime>,
    client: Arc<Client>,
}

impl GetSecretValueJson {
    pub fn new(runtime: &Arc<Runtime>, client: &Arc<Client>) -> Self {
        GetSecretValueJson {
            runtime: runtime.clone(),
            client: client.clone(),
        }
    }
}

impl Function for GetSecretValueJson {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        map_secret_string(
            "secretsmanager_get_secret_value_json",
            &self.runtime,
            &self.client,
            args,
            |secret_id, secret_string| {
                secret_string
                    .map_or(Ok(Value::Null), |s| from_str::<Value>(s))
                    .map_err(|err| into_tera_error(secret_id, err))
            },
        )
    }
}

fn map_secret_string<F>(
    name: &str,
    runtime: &Arc<Runtime>,
    client: &Arc<Client>,
    args: &HashMap<String, Value>,
    f: F,
) -> tera::Result<Value>
where
    F: FnOnce(&String, Option<&str>) -> tera::Result<Value>,
{
    let secret_id = args.get("secret_id").ok_or_else(|| {
        tera::Error::msg(format!(
            "Function `{}` didn't receive a `secret_id` argument",
            name
        ))
    })?;
    let secret_id: String = from_value(secret_id.clone()).map_err(|_| {
        tera::Error::msg(format!(
            "Function `{}` received secret_id={} but `secret_id` can only be a string",
            name, secret_id
        ))
    })?;
    let future = client.get_secret_value().secret_id(&secret_id).send();
    f(
        &secret_id,
        runtime
            .block_on(future)
            .map_err(|err| into_tera_error(&secret_id, err))?
            .secret_string(),
    )
}

fn into_tera_error<D: std::fmt::Display>(secret_id: &str, display: D) -> tera::Error {
    tera::Error::msg(format!("{}: {}", secret_id, display))
}
