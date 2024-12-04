#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]


use crate::request::{make_request, generate_json};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Functions for accessing cryptocurrency-related data from the FMP API
pub struct Cryptos;

impl Cryptos {
    /// Lists all available cryptocurrencies.
    /// 
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn list() -> Result<Value, reqwest::Error> {
        make_request("symbol/available-cryptocurrencies", HashMap::new()).await
    }

    /// Ensures a cryptocurrency symbol ends with 'USD'.
    /// 
    /// ## Arguments
    /// 
    /// * `symbol` - A &str that contains the symbol of the cryptocurrency to normalize.
    ///
    /// ## Returns
    ///
    /// A String containing the normalized symbol.
    fn normalize_symbol(symbol: &str) -> String {
        let symbol = symbol.to_uppercase();
        if !symbol.to_lowercase().contains("usd") {
            format!("{}USD", symbol)
        } else {
            symbol
        }
    }

    /// Gets quotes for either specific cryptocurrencies or all cryptocurrencies.
    ///
    /// ## Arguments
    ///
    /// * `symbols` - Optional single symbol or vector of symbols to get quotes for. If None, returns quotes for all cryptocurrencies.
    ///               If Some, can be either a single symbol as &str or multiple symbols as &[&str].
    ///               Symbols will be automatically normalized to include 'USD' suffix if missing.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response with cryptocurrency quotes or an error.
    ///
    /// ## Examples
    ///
    /// ```
    /// // Get quote for a single cryptocurrency
    /// let btc_quote = Cryptos::quote(Some(Either::Single("BTC"))).await?;
    ///
    /// // Get quotes for multiple cryptocurrencies
    /// let quotes = Cryptos::quote(Some(Either::Array(&["BTC", "ETH"]))).await?;
    ///
    /// // Get quotes for all cryptocurrencies
    /// let all_quotes = Cryptos::quote(None).await?;
    /// ```
    pub async fn quote(symbols: Option<Either<&str, &[&str]>>) -> Result<Value, reqwest::Error> {
        match symbols {
            Some(Either::Single(symbol)) => {
                let normalized = Self::normalize_symbol(symbol);
                make_request("quote", generate_json(Value::String(normalized), None)).await
            }
            Some(Either::Array(symbols)) => {
                let normalized: Vec<String> = symbols.iter()
                    .map(|s| Self::normalize_symbol(s))
                    .collect();
                make_request("quote", generate_json(Value::Array(
                    normalized.iter().map(|s| Value::String(s.clone())).collect()
                ), None)).await
            }
            None => make_request("cryptocurrencies", HashMap::new()).await
        }
    }

    /// Gets historical price data for cryptocurrencies.
    ///
    /// ## Arguments
    ///
    /// * `symbols` - Single symbol or vector of symbols to get history for
    /// * `start_date` - Optional start date
    /// * `end_date` - Optional end date
    /// * `data_type` - Optional data type
    /// * `limit` - Optional limit of data points
    /// 
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn history(
        symbols: Either<&str, &[&str]>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        data_type: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Value, reqwest::Error> {
        let normalized = match symbols {
            Either::Single(symbol) => Value::String(Self::normalize_symbol(symbol)),
            Either::Array(symbols) => Value::Array(
                symbols.iter()
                    .map(|s| Value::String(Self::normalize_symbol(s)))
                    .collect()
            ),
        };

        let query_params = json!({
            "from": start_date,
            "to": end_date,
            "serietype": data_type,
            "timeseries": limit
        });

        make_request(
            "historical-price-full/crypto",
            generate_json(normalized, Some(query_params))
        ).await
    }
}

/// Helper enum to handle single value or array of values.
pub enum Either<L, R> {
    Single(L),
    Array(R),
}



pub async fn example() -> Result<(), reqwest::Error> {
    // List all cryptocurrencies
    let cryptos = Cryptos::list().await?;
    
    // Get quote for a single crypto
    let btc_quote = Cryptos::quote(Some(Either::Single("BTC"))).await?;
    
    // Get quotes for multiple cryptos
    let quotes = Cryptos::quote(Some(Either::Array(&["BTC", "ETH"]))).await?;
    
    // Get historical data
    let history = Cryptos::history(
        Either::Single("BTC"),
        Some("2023-01-01"),
        Some("2023-12-31"),
        None,
        Some(100)
    ).await?;
    
    Ok(())
}