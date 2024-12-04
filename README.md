```sh
# Market Data API Client

## Overview

The Market Data API Client is a Rust-based application that interacts with the Financial Modeling Prep (FMP) API to retrieve various financial data, including stock quotes, market indices, commodities, cryptocurrencies, and more. This project is designed to provide a simple and efficient way to access financial data programmatically.

## Features

- Retrieve stock quotes and historical data
- Access market indices and their quotes
- Get information on commodities and cryptocurrencies
- Fetch financial statements and metrics for companies
- Support for asynchronous operations using Tokio

## Getting Started

### Prerequisites

- Rust (version 1.30 or higher)
- Cargo (Rust package manager)
- An API key from Financial Modeling Prep (FMP)

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/market_data.git
   cd market_data
   ```

2. Create a `.env` file in the root directory and add your FMP API key:

   ```plaintext
   FMP_API_KEY=your_api_key_here
   ```

3. Build the project:

   ```bash
   cargo build
   ```

4. Run the application:

   ```bash
   cargo run
   ```

## Usage

The application provides various modules to interact with different aspects of the FMP API. Here are some examples of how to use the client:

### Example: Fetching Most Active Stocks

```

```sh

## Modules

- **auth**: Handles API key retrieval from environment variables.
- **market**: Provides functions to access market data.
- **financial**: Contains methods for fetching financial statements.
- **crypto**: Interacts with cryptocurrency data.
- **forex**: Accesses foreign exchange rate data.
- **commodity**: Retrieves commodity-related information.
- **etf**: Provides access to ETF data.
- **mutualfund**: Interacts with mutual fund data.
- **index**: Accesses market index data.
- **search**: Allows searching for financial instruments.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Financial Modeling Prep](https://financialmodelingprep.com/) for providing the API.
- [Tokio](https://tokio.rs/) for asynchronous programming in Rust.
```