#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use std::collections::HashMap;

use crate::request::{make_request, generate_json};
use crate::financial::Financial;
use serde_json::{json, Value};

/// Functions for accessing stock-related data from the FMP API.
pub struct Stock<'a> {
    symbol: &'a str,
}

impl<'a> Stock<'a> {
    /// Creates a new Stock instance for a specific stock symbol.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - The stock symbol to get data for
    pub fn new(symbol: &'a str) -> Self {
        Self { symbol }
    }

    pub async fn list() -> Result<Value, reqwest::Error> {
        make_request("stock/list", HashMap::new()).await
    }
    /// Gets company profile information.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn profile(&self) -> Result<Value, reqwest::Error> {
        make_request(
            "company/profile",
            generate_json(Value::String(self.symbol.to_string()), None)
        ).await
    }

    /// Gets current stock quote.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn quote(&self) -> Result<Value, reqwest::Error> {
        make_request(
            "quote",
            generate_json(Value::String(self.symbol.to_string()), None)
        ).await
    }

    /// Gets financial statements and metrics.
    ///
    /// ## Returns
    ///
    /// A Financials instance for accessing financial data.
    pub fn financial(&self) -> Financial {
        Financial::new(self.symbol)
    }

    /// Gets company rating information.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn rating(&self) -> Result<Value, reqwest::Error> {
        make_request(
            "company/rating",
            generate_json(Value::String(self.symbol.to_string()), None)
        ).await
    }

    /// Gets real-time stock price.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn current_price(&self) -> Result<Value, reqwest::Error> {
        make_request(
            "stock/real-time-price",
            generate_json(Value::String(self.symbol.to_string()), None)
        ).await
    }

    /// Gets historical price data.
    ///
    /// ## Arguments
    ///
    /// * `start_date` - Optional start date
    /// * `end_date` - Optional end date
    /// * `data_type` - Optional data type
    /// * `limit` - Optional limit of data points
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn history(
        &self,
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
            "historical-price-full",
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets dividend history.
    ///
    /// ## Arguments
    ///
    /// * `start_date` - Optional start date
    /// * `end_date` - Optional end date
    /// * `data_type` - Optional data type
    /// * `limit` - Optional limit of data points
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn dividend_history(
        &self,
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
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets stock split history.
    ///
    /// ## Arguments
    ///
    /// * `start_date` - Optional start date
    /// * `end_date` - Optional end date
    /// * `data_type` - Optional data type
    /// * `limit` - Optional limit of data points
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn split_history(
        &self,
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
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }
}


pub async fn example() -> Result<(), reqwest::Error> {
    let aapl = Stock::new("AAPL");
    
    // Get company profile
    let profile = aapl.profile().await?;
    
    // Get current quote
    let quote = aapl.quote().await?;
    
    // Get financial data
    let financials = aapl.financial();
    let income = financials.income(None).await?;
    
    // Get historical data
    let history = aapl.history(
        Some("2023-01-01"),
        Some("2023-12-31"),
        None,
        Some(100)
    ).await?;
    
    Ok(())
}
