#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use crate::requests::make_request;
use serde_json::Value;
use std::collections::HashMap;

/// Functions for accessing general market data from the FMP API.
pub struct Market;

impl Market {
    /// Gets the most active stocks in the market.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn most_active() -> Result<Value, reqwest::Error> {
        make_request("stock/actives", HashMap::new()).await
    }

    /// Gets the stocks with the highest gains.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn most_gainer() -> Result<Value, reqwest::Error> {
        make_request("stock/gainers", HashMap::new()).await
    }

    /// Gets the stocks with the highest losses.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn most_loser() -> Result<Value, reqwest::Error> {
        make_request("stock/losers", HashMap::new()).await
    }

    /// Gets the performance of different market sectors.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn sector_performance() -> Result<Value, reqwest::Error> {
        make_request("stock/sectors-performance", HashMap::new()).await
    }

    /// Gets the current market trading hours status.
    ///
    /// ## Returns
    ///
    /// A Result containing either the JSON response or an error.
    pub async fn trading_hours() -> Result<Value, reqwest::Error> {
        make_request("is-the-market-open", HashMap::new()).await
    }
}


async fn example() -> Result<(), reqwest::Error> {
    // Get most active stocks
    let active = Market::most_active().await?;
    
    // Get top gainers
    let gainers = Market::most_gainer().await?;
    
    // Get sector performance
    let sectors = Market::sector_performance().await?;
    
    // Check if market is open
    let is_open = Market::trading_hours().await?;
    
    Ok(())
}

