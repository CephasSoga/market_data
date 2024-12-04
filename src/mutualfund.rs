#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use crate::request::{make_request, generate_json};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Functions for accessing mutual fund data from the FMP API.
pub struct MutualFund;

impl MutualFund {
    /// Lists all available mutual funds.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn list() -> Result<Value, reqwest::Error> {
        make_request("symbol/available-mutual-funds", HashMap::new()).await
    }

    /// Gets quotes for either a specific mutual fund or all mutual funds.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - Optional mutual fund symbol to get quote for. If None, returns quotes for all mutual funds.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn quote(symbol: Option<&str>) -> Result<Value, reqwest::Error> {
        match symbol {
            Some(s) => make_request("quote", generate_json(Value::String(s.to_string()), None)).await,
            None => make_request("quotes/mutual_fund", HashMap::new()).await,
        }
    }

    /// Gets historical price data for a mutual fund.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - Mutual fund symbol to get history for
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
            "historical-price-full/mutual_fund",
            generate_json(Value::String(symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets dividend history for a mutual fund.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - Mutual fund symbol to get dividend history for
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

    /// Gets split history for a mutual fund.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - Mutual fund symbol to get split history for
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
    // List all mutual funds
    let funds = MutualFund::list().await?;
    
    // Get quote for a specific fund
    let vanguard_quote = MutualFund::quote(Some("VFINX")).await?;
    
    // Get historical data
    let history = MutualFund::history(
        "VFINX",
        Some("2023-01-01"),
        Some("2023-12-31"),
        None,
        Some(100)
    ).await?;
    
    Ok(())
}
