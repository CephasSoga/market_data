#![allow(dead_code)]
#![allow(warnings)]
#![allow(unused_variables)]

use std::env;
use dotenv::dotenv;

/// Retrieves the Financial Modeling Prep API key from the environment variables.
///
/// ## Returns
///
/// A String containing the API key.
pub fn fmp_api_key() -> String {
    dotenv().ok(); // Load the .env file
    match env::var("FMP_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: FMP_API_KEY environment variable not found");
            panic!("FMP_API_KEY environment variable not found");
        }
    }
}
