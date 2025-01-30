#[derive(Debug, Clone)]
pub enum FetchType {
    Crypto,
    Forex,
    Stock,
    Unknown
}
impl FetchType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "crypto" => Self::Crypto,
            "forex"  => Self::Forex,
            "stock" => Self::Stock,
            _ => Self::Unknown
        }
    }
    
}