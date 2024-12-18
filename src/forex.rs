#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use std::collections::HashMap;

use crate::request::{make_request, generate_json};
use serde_json::{json, Value};

/// Functions for accessing foreign exchange rate data from the FMP API.
pub struct Forex<'a> {
    from_curr: &'a str,
    to_curr: &'a str,
}

impl<'a> Forex<'a> {
    /// Creates a new Forex instance for a specific currency pair.
    ///
    /// ## Arguments
    ///
    /// * `from_curr` - The base currency code
    /// * `to_curr` - The quote currency code
    pub fn new(from_curr: &'a str, to_curr: &'a str) -> Self {
        Self { 
            from_curr,
            to_curr,
        }
    }

    pub async fn list() -> Result<Value, reqwest::Error> {
        make_request("symbol/available-forex-currency-pairs", HashMap::new()).await
    }

    /// Gets the current exchange rate for the currency pair.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn rate(&self) -> Result<Value, reqwest::Error> {
        let symbol = format!("{}{}", self.from_curr, self.to_curr);
        make_request(
            "quote",
            generate_json(Value::String(symbol), None)
        ).await
    }

    /// Gets historical exchange rate data for the currency pair.
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
        let symbol = format!("{}{}", self.from_curr, self.to_curr);
        let query_params = json!({
            "from": start_date,
            "to": end_date,
            "serietype": data_type,
            "timeseries": limit
        });

        make_request(
            "historical-price-full/forex",
            generate_json(Value::String(symbol), Some(query_params))
        ).await
    }
}


pub async fn example() -> Result<(), reqwest::Error> {
    let eur_usd = Forex::new("EUR", "USD");
    
    // Get current exchange rate
    let rate = eur_usd.rate().await?;
    
    // Get historical exchange rates
    let history = eur_usd.history(
        Some("2023-01-01"),
        Some("2023-12-31"),
        None,
        Some(100)
    ).await?;
    
    Ok(())
}