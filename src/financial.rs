#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use crate::request::{make_request, generate_json};
use serde_json::{json, Value};

/// Functions for accessing financial statement data from the FMP API
pub struct Financial<'a> {
    symbol: &'a str,
}

impl<'a> Financial<'a> {
    /// Creates a new Financials instance for a specific stock symbol.
    ///
    /// ## Arguments
    ///
    /// * `symbol` - The stock symbol to get financial data for
    pub fn new(symbol: &'a str) -> Self {
        Self { symbol }
    }

    /// Gets income statement data.
    ///
    /// ## Arguments
    ///
    /// * `period` - The period for the data (defaults to "annual")
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn income(&self, period: Option<&str>) -> Result<Value, reqwest::Error> {
        let query_params = json!({
            "period": period.unwrap_or("annual")
        });

        make_request(
            "financials/income-statement",
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets balance sheet data.
    ///
    /// ## Arguments
    ///
    /// * `period` - The period for the data (defaults to "annual")
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn balance(&self, period: Option<&str>) -> Result<Value, reqwest::Error> {
        let query_params = json!({
            "period": period.unwrap_or("annual")
        });

        make_request(
            "financials/balance-sheet-statement",
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets cash flow statement data.
    ///
    /// ## Arguments
    ///
    /// * `period` - The period for the data (defaults to "annual")
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn cashflow(&self, period: Option<&str>) -> Result<Value, reqwest::Error> {
        let query_params = json!({
            "period": period.unwrap_or("annual")
        });

        make_request(
            "financials/cash-flow-statement",
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets key company metrics.
    ///
    /// ## Arguments
    ///
    /// * `period` - The period for the data (defaults to "annual")
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn metrics(&self, period: Option<&str>) -> Result<Value, reqwest::Error> {
        let query_params = json!({
            "period": period.unwrap_or("annual")
        });

        make_request(
            "company-key-metrics",
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets financial statement growth data.
    ///
    /// ## Arguments
    ///
    /// * `period` - The period for the data (defaults to "annual")
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn growth(&self, period: Option<&str>) -> Result<Value, reqwest::Error> {
        let query_params = json!({
            "period": period.unwrap_or("annual")
        });

        make_request(
            "financial-statement-growth",
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets company enterprise value data.
    ///
    /// ## Arguments
    ///
    /// * `period` - The period for the data (defaults to "annual")
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn company_value(&self, period: Option<&str>) -> Result<Value, reqwest::Error> {
        let query_params = json!({
            "period": period.unwrap_or("annual")
        });

        make_request(
            "enterprise-value",
            generate_json(Value::String(self.symbol.to_string()), Some(query_params))
        ).await
    }

    /// Gets financial ratios.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn ratios(&self) -> Result<Value, reqwest::Error> {
        make_request(
            "financial-ratios",
            generate_json(Value::String(self.symbol.to_string()), None)
        ).await
    }
}


pub async fn example() -> Result<(), reqwest::Error> {
    let aapl = Financial::new("AAPL");
    
    // Get annual income statement
    let income = aapl.income(None).await?;
    
    // Get quarterly balance sheet
    let balance = aapl.balance(Some("quarter")).await?;
    
    // Get financial ratios
    let ratios = aapl.ratios().await?;
    
    Ok(())
}