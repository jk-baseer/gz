use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BidRequest {
    pub id: String,
    pub imp: Vec<Imp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<Site>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<App>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<Device>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    /// Auction type: 1 = first price, 2 = second price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<u32>,
    /// Max time in ms for response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmax: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cur: Option<Vec<String>>,
    /// Blocked advertiser categories (IAB)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcat: Option<Vec<String>>,
    /// Blocked advertiser domains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badv: Option<Vec<String>>,
    /// 1 = test mode (bids not billable)
    #[serde(default)]
    pub test: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Imp {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Banner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<Video>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native: Option<Native>,
    /// Publisher's ad tag ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagid: Option<String>,
    /// Bid floor in the currency specified by bidfloorcur
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bidfloor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bidfloorcur: Option<String>,
    /// 1 = interstitial
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instl: Option<u8>,
    /// 1 = HTTPS required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Banner {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<Format>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    /// Blocked creative types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub btype: Option<Vec<u32>>,
    /// Blocked creative attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battr: Option<Vec<u32>>,
    /// Ad position (0=unknown, 1=above fold, 3=below fold, 4=header, 5=footer, 6=sidebar, 7=full-screen)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Format {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Video {
    pub mimes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minduration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxduration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linearity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocols: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Native {
    /// JSON-encoded NativeRequest (OpenRTB Native 1.2)
    pub request: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

/// Parsed native ad request (subset of OpenRTB Native 1.2 used for matching)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NativeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ver: Option<String>,
    pub assets: Vec<NativeAsset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NativeAsset {
    pub id: u32,
    #[serde(default)]
    pub required: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<NativeTitle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub img: Option<NativeImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<NativeData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NativeTitle {
    pub len: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NativeImage {
    /// 1=Icon, 3=Main image
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wmin: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hmin: Option<u32>,
    pub mimes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NativeData {
    /// 1=Sponsored, 2=Desc, 11=CTA text, 12=Rating, 500+=custom
    #[serde(rename = "type")]
    pub type_: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub len: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Site {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// IAB content categories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cat: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Publisher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct App {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// App bundle (e.g. com.example.app)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storeurl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cat: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Publisher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Publisher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cat: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Device {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ua: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<Geo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Device type: 1=Mobile/Tablet, 2=PC, 3=Connected TV, 4=Phone, 5=Tablet, 6=Connected Device, 7=Set Top Box
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devicetype: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub make: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
    /// Connection type: 0=Unknown, 1=Ethernet, 2=WiFi, 3=Cellular, 4=LTE, 5=5G
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connectiontype: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ifa: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Geo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
    /// ISO-3166-1-alpha-3 country code (e.g. "ARE", "SAU", "KWT")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
    /// 1=GPS, 2=IP, 3=User-provided
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyeruid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yob: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<Geo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<serde_json::Value>,
}

/// OpenRTB device type constants
pub mod device_type {
    pub const MOBILE_TABLET: u32 = 1;
    pub const PC: u32 = 2;
    pub const CONNECTED_TV: u32 = 3;
    pub const PHONE: u32 = 4;
    pub const TABLET: u32 = 5;
    pub const CONNECTED_DEVICE: u32 = 6;
    pub const SET_TOP_BOX: u32 = 7;
}
