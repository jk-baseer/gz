use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidResponse {
    /// Must echo the BidRequest.id
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub seatbid: Vec<SeatBid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bidid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cur: Option<String>,
    /// No-bid reason code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbr: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

impl BidResponse {
    pub fn no_bid(request_id: impl Into<String>) -> Self {
        Self {
            id: request_id.into(),
            seatbid: vec![],
            bidid: None,
            cur: None,
            nbr: Some(NoBidReason::NoMatchingImp as u32),
            ext: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatBid {
    pub bid: Vec<Bid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seat: Option<String>,
    /// 1 = all bids in this seat must win or none
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    /// Unique ID for this bid
    pub id: String,
    /// Must match Imp.id from the request
    pub impid: String,
    /// CPM bid price in the request currency
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adid: Option<String>,
    /// Win notice URL — exchange calls this when we win
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nurl: Option<String>,
    /// Billing notice URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burl: Option<String>,
    /// Ad markup (HTML/JS for banner)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adm: Option<String>,
    /// Advertiser domains for block list checking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adomain: Option<Vec<String>>,
    /// Creative ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cat: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    /// Bid expiry in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

/// Standard no-bid reason codes
#[repr(u32)]
pub enum NoBidReason {
    UnknownError = 0,
    TechnicalError = 1,
    InvalidRequest = 2,
    KnownWebSpider = 3,
    SuspectedNonhumanTraffic = 4,
    CloudOrDatacenterTraffic = 5,
    UnsupportedDevice = 6,
    BlockedPublisher = 7,
    UnmatchedUser = 8,
    DailyReaderCapMet = 9,
    DailyDomainCapMet = 10,
    NoMatchingImp = 300,
    BudgetExhausted = 301,
    NoCampaignsActive = 302,
}
