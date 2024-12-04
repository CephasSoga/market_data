#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use crate::request::{make_request, generate_json};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Functions for accessing market index data from the FMP API.
pub struct Index;

impl Index {
    /// Lists all available market indices.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn list() -> Result<Value, reqwest::Error> {
        make_request("symbol/available-indexes", HashMap::new()).await
    }

    /// Gets quotes for either a specific index or all indices.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - Optional index symbol to get quote for. If None, returns quotes for all indices.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn quote(symbol: Option<&str>) -> Result<Value, reqwest::Error> {
        match symbol {
            Some(s) => make_request("quote", generate_json(Value::String(s.to_string()), None)).await,
            None => make_request("quotes/index", HashMap::new()).await,
        }
    }

    /// Gets historical price data for an index.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - Index symbol to get history for
    /// * `start_date` - Optional start date
    /// * `end_date` - Optional end date
    /// * `data_type` - Optional data type
    /// * `limit` - Optional limit of data points
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn history(
        symbol: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        data_type: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Value, reqwest::Error> {
        let query_params = json!({
            "from": start_date,
            "to": end_date,
            "serietype": data_type,
            "timeseries": limit
        });

        make_request(
            "historical-price-full/index",
            generate_json(Value::String(symbol.to_string()), Some(query_params))
        ).await
    }
}


pub async fn example() -> Result<(), reqwest::Error> {
    // List all indices
    let indices = Index::list().await?;
    
    // Get quote for a specific index
    let sp500_quote = Index::quote(Some("^GSPC")).await?;
    
    // Get quotes for all indices
    let all_quotes = Index::quote(None).await?;
    
    // Get historical data
    let history = Index::history(
        "^GSPC",
        Some("2023-01-01"),
        Some("2023-12-31"),
        None,
        Some(100)
    ).await?;
    
    Ok(())
}