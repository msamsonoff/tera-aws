use std::collections::HashMap;
use std::sync::Arc;

use aws_config::imds::client::Client;
use eyre::Result;
use serde_json::value::from_value;
use serde_json::Value;
use tera::{Function, Tera};
use tokio::runtime::Runtime;

pub fn register(tera: &mut Tera, runtime: &Arc<Runtime>) -> Result<()> {
    let imds = Imds::new(runtime)?;
    tera.register_function("imds", imds);
    Ok(())
}

pub struct Imds {
    runtime: Arc<Runtime>,
    client: Client,
}

impl Imds {
    pub fn new(runtime: &Arc<Runtime>) -> Result<Self> {
        let future = get_imds_client();
        let client = runtime.block_on(future)?;
        let runtime = runtime.clone();
        let imds = Imds { runtime, client };
        Ok(imds)
    }
}

async fn get_imds_client() -> Result<Client> {
    let client = Client::builder().build().await?;
    Ok(client)
}

impl Function for Imds {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let path = args
            .get("path")
            .ok_or_else(|| tera::Error::msg("Function `imds` didn't receive a `path` argument"))?;
        let path: String = from_value(path.clone()).map_err(|_| {
            tera::Error::msg(format!(
                "Function `imds` received path={} but `path` can only be a string",
                path
            ))
        })?;
        let future = self.client.get(&path);
        let response = self
            .runtime
            .block_on(future)
            .map_err(|err| tera::Error::msg(format!("{}", err)))?;
        let value = Value::String(response);
        Ok(value)
    }
}
