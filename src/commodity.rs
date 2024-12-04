use crate::request::{make_request, generate_json};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Functions for accessing commodity-related data from the FMP API
pub struct Commodities;

impl Commodities {
    /// Lists all available commodities.
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn list() -> Result<Value, reqwest::Error> {
        make_request("symbol/available-commodities", HashMap::new()).await
    }

    /// Gets quotes for either a specific commodity or all commodities.
    /// 
    /// ## Arguments
    ///
    /// * `symbol` - An Option<&str> that contains the symbol of the commodity to get quotes for.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn quote(symbol: Option<&str>) -> Result<Value, reqwest::Error> {
        match symbol {
            Some(s) => make_request("quote", generate_json(Value::String(s.to_string()), None)).await,
            None => make_request("quotes/commodity", HashMap::new()).await,
        }
    }

    /// Gets historical price data for a commodity.
    /// 
    /// ## Arguments
    ///
    /// * `symbol` - A &str that contains the symbol of the commodity to get historical price data for.
    /// * `start_date` - An Option<&str> that contains the start date of the historical price data to get.
    /// * `end_date` - An Option<&str> that contains the end date of the historical price data to get.
    /// * `data_type` - An Option<&str> that contains the type of data to get.
    /// * `limit` - An Option<i32> that contains the maximum number of historical price data points to get.
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
            "historical-price-full/commodity",
            generate_json(Value::String(symbol.to_string()), Some(query_params))
        ).await
    }
} 