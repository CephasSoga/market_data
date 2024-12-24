#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use crate::api_calls::request::{make_request, generate_json};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Functions for accessing ETF-related data from the FMP API
pub struct Etf;

impl Etf {
    /// Lists all available ETFs.
    /// 
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn list() -> Result<Value, reqwest::Error> {
        make_request("symbol/available-etfs", HashMap::new()).await
    }

    /// Gets quotes for either a specific ETF or all ETFs.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - Optional ETF symbol to get quote for. If None, returns quotes for all ETFs.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn quote(symbol: Option<&str>) -> Result<Value, reqwest::Error> {
        match symbol {
            Some(s) => make_request("quote", generate_json(Value::String(s.to_string()), None)).await,
            None => make_request("quotes/etf", HashMap::new()).await,
        }
    }

    /// Gets historical price data for an ETF.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - ETF symbol to get history for
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
            "historical-price-full/etf",
            generate_json(Value::String(symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets dividend history for an ETF.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - ETF symbol to get dividend history for
    /// * `start_date` - Optional start date
    /// * `end_date` - Optional end date
    /// * `data_type` - Optional data type
    /// * `limit` - Optional limit of data points
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn dividend_history(
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
            "historical-price-full/stock_dividend",
            generate_json(Value::String(symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets split history for an ETF.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - ETF symbol to get split history for
    /// * `start_date` - Optional start date
    /// * `end_date` - Optional end date
    /// * `data_type` - Optional data type
    /// * `limit` - Optional limit of data points
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn split_history(
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
            "historical-price-full/stock_split",
            generate_json(Value::String(symbol.to_string()), Some(query_params))
        ).await
    }
}


pub async fn example() -> Result<(), reqwest::Error> {
    // List all ETFs
    let etfs = Etf::list().await?;
    
    // Get quote for a specific ETF
    let spy_quote = Etf::quote(Some("SPY")).await?;
    
    // Get historical data
    let history = Etf::history(
        "SPY",
        Some("2023-01-01"),
        Some("2023-12-31"),
        None,
        Some(100)
    ).await?;
    
    Ok(())
}