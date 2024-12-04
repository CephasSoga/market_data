#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use reqwest;
use serde_json::Value;
use std::collections::HashMap;
use crate::auth;

/// The base URL for the Financial Modeling Prep API.
const BASE_URL: &str = "https://financialmodelingprep.com/api/v3/";

/// Creates the endpoint using the default base URL and adding the respective
/// path and query parameters to be requested by the API.
///
/// Future Use: In case of querying a new database in the future, this is the
/// only function that would require modification for the most part to control how.
/// data is queried.
///
/// ## Arguments
///
/// * `params` - A HashMap containing query and path parameters to be passed to the API.
///
/// ## Returns
///
/// A String that represents the REST API request to be made to FMP API.
fn url(params: &HashMap<String, Value>) -> String {
    let query_parameters = params.get("query").and_then(|v| v.as_object());
    let path_parameters = params.get("path");
    let url_path = params.get("urlPath").and_then(|v| v.as_str()).unwrap_or("");
    let mut url = String::from(BASE_URL);
    let mut query_string = String::new();
    let apikey = auth::fmp_api_key();

    url.push_str(url_path);

    if let Some(path_params) = path_parameters {
        let path_str = match path_params {
            Value::Array(arr) => arr.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","),
            _ => path_params.to_string(),
        };
        url.push('/');
        url.push_str(&path_str.to_uppercase());
    }

    if let Some(query_params) = query_parameters {
        for (key, value) in query_params {
            if !value.is_null() {
                query_string.push_str(&format!("{}={}&", key, value));
            }
        }
        if !query_string.is_empty() {
            query_string.pop(); // Remove trailing '&'
            url.push('?');
            url.push_str(&query_string);
        }
    }

    if query_string.is_empty() {
        url.push('?');
    } else {
        url.push('&');
    }

    url.push_str(&format!("apikey={}", apikey));

    url
}

/// Generates a nested JSON Object to accommodate all the necessary parameters.
/// that are going to be passed to FMP's REST API.
///
/// # Arguments
///
/// * `path_param` - A Value that represents the path of the endpoint.
/// * `query_param` - An Option<Value> that contains key-value pairs to be added to the query parameters.
///
/// ## Returns
///
/// A HashMap that contains the respective query and path parameters which will be parsed by `url` function.
pub fn generate_json(path_param: Value, query_param: Option<Value>) -> HashMap<String, Value> {
    let mut params = HashMap::new();
    params.insert("query".to_string(), query_param.unwrap_or(Value::Null));
    params.insert("path".to_string(), path_param);
    params
}

/// Makes a GET request to the specified path with the given parameters.
///
/// ## Arguments
///
/// * `path` - A string slice that holds the API endpoint path.
/// * `params` - A HashMap containing additional parameters for the request.
///
/// ## Returns
///
/// A Result containing either the JSON response or an error.
pub async fn make_request(path: &str, params: HashMap<String, Value>) -> Result<Value, reqwest::Error> {
    let mut request_params = params;
    request_params.insert("urlPath".to_string(), Value::String(path.to_string()));
    let url = url(&request_params);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    response.json::<Value>().await
}