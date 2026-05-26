use bidder::targeting::matches;
use bidder::index::{CampaignRecord, CreativeRecord, TargetingRecord};
use chrono::Utc;
use openrtb::{BidRequest, Banner, Device, Format, Geo, Imp, Site};
use uuid::Uuid;

fn campaign(targeting: TargetingRecord) -> CampaignRecord {
    CampaignRecord {
        id: Uuid::new_v4(),
        advertiser_id: Uuid::new_v4(),
        name: "test".into(),
        bid_price_cpm_cents: 200,
        budget_daily_cents: Some(10_000_00),
        budget_total_cents: 100_000_00,
        start_date: None,
        end_date: None,
        targeting,
        creatives: vec![CreativeRecord {
            id: Uuid::new_v4(),
            format: "banner".into(),
            width: Some(300),
            height: Some(250),
            asset_url: "https://cdn.test/ad.jpg".into(),
            click_url: "https://advertiser.test/landing".into(),
        }],
    }
}

fn basic_req() -> BidRequest {
    BidRequest {
        id: "auction-1".into(),
        imp: vec![Imp {
            id: "imp-1".into(),
            banner: Some(Banner {
                format: Some(vec![Format { w: 300, h: 250 }]),
                w: Some(300),
                h: Some(250),
                btype: None,
                battr: None,
                pos: None,
                ext: None,
            }),
            video: None,
            native: None,
            tagid: None,
            bidfloor: None,
            bidfloorcur: None,
            instl: None,
            secure: None,
            ext: None,
        }],
        site: Some(Site {
            id: None,
            name: None,
            domain: Some("news.example.ae".into()),
            cat: Some(vec!["IAB13".into()]),
            page: None,
            referrer: None,
            publisher: None,
            ext: None,
        }),
        app: None,
        device: Some(Device {
            ua: None,
            geo: Some(Geo {
                lat: None,
                lon: None,
                country: Some("ARE".into()),
                region: None,
                city: None,
                zip: None,
                type_: None,
            }),
            ip: None,
            devicetype: Some(4), // phone
            make: None,
            model: None,
            os: Some("iOS".into()),
            osv: None,
            language: Some("ar".into()),
            carrier: None,
            connectiontype: None,
            ifa: None,
            ext: None,
        }),
        user: None,
        at: Some(1),
        tmax: Some(100),
        cur: None,
        bcat: None,
        badv: None,
        test: 0,
        ext: None,
    }
}

fn imp() -> Imp {
    basic_req().imp.into_iter().next().unwrap()
}

#[test]
fn no_targeting_constraints_always_matches() {
    let c = campaign(TargetingRecord::default());
    let req = basic_req();
    assert!(matches(&c, &req, &imp()));
}

#[test]
fn geo_match_passes() {
    let c = campaign(TargetingRecord {
        geo_countries: vec!["ARE".into()],
        ..Default::default()
    });
    assert!(matches(&c, &basic_req(), &imp()));
}

#[test]
fn geo_mismatch_fails() {
    let c = campaign(TargetingRecord {
        geo_countries: vec!["SAU".into()],
        ..Default::default()
    });
    assert!(!matches(&c, &basic_req(), &imp()));
}

#[test]
fn geo_no_device_fails_when_geo_required() {
    let mut req = basic_req();
    req.device = None;
    let c = campaign(TargetingRecord {
        geo_countries: vec!["ARE".into()],
        ..Default::default()
    });
    assert!(!matches(&c, &req, &imp()));
}

#[test]
fn device_type_mobile_matches_phone() {
    let c = campaign(TargetingRecord {
        device_types: vec!["mobile".into()],
        ..Default::default()
    });
    // devicetype=4 (phone) maps to "mobile"
    assert!(matches(&c, &basic_req(), &imp()));
}

#[test]
fn device_type_desktop_fails_phone() {
    let c = campaign(TargetingRecord {
        device_types: vec!["desktop".into()],
        ..Default::default()
    });
    assert!(!matches(&c, &basic_req(), &imp()));
}

#[test]
fn os_ios_matches() {
    let c = campaign(TargetingRecord {
        os_types: vec!["ios".into()],
        ..Default::default()
    });
    assert!(matches(&c, &basic_req(), &imp()));
}

#[test]
fn os_android_fails_ios_device() {
    let c = campaign(TargetingRecord {
        os_types: vec!["android".into()],
        ..Default::default()
    });
    assert!(!matches(&c, &basic_req(), &imp()));
}

#[test]
fn language_ar_matches() {
    let c = campaign(TargetingRecord {
        languages: vec!["ar".into()],
        ..Default::default()
    });
    assert!(matches(&c, &basic_req(), &imp()));
}

#[test]
fn language_en_fails_ar_device() {
    let c = campaign(TargetingRecord {
        languages: vec!["en".into()],
        ..Default::default()
    });
    assert!(!matches(&c, &basic_req(), &imp()));
}

#[test]
fn iab_category_matches() {
    let c = campaign(TargetingRecord {
        site_categories: vec!["IAB13".into()],
        ..Default::default()
    });
    assert!(matches(&c, &basic_req(), &imp()));
}

#[test]
fn iab_category_mismatch_fails() {
    let c = campaign(TargetingRecord {
        site_categories: vec!["IAB1".into()],
        ..Default::default()
    });
    assert!(!matches(&c, &basic_req(), &imp()));
}

#[test]
fn bid_floor_above_our_price_fails() {
    let c = campaign(TargetingRecord::default());
    // Our bid = $2.00 CPM (200 cents). Floor = $3.00.
    let mut i = imp();
    i.bidfloor = Some(3.0);
    assert!(!matches(&c, &basic_req(), &i));
}

#[test]
fn bid_floor_at_our_price_passes() {
    let c = campaign(TargetingRecord::default());
    let mut i = imp();
    i.bidfloor = Some(2.0);
    assert!(matches(&c, &basic_req(), &i));
}

#[test]
fn expired_campaign_fails() {
    let mut c = campaign(TargetingRecord::default());
    c.end_date = Some(Utc::now() - chrono::Duration::hours(1));
    assert!(!matches(&c, &basic_req(), &imp()));
}

#[test]
fn future_campaign_fails() {
    let mut c = campaign(TargetingRecord::default());
    c.start_date = Some(Utc::now() + chrono::Duration::hours(1));
    assert!(!matches(&c, &basic_req(), &imp()));
}

#[test]
fn creative_size_mismatch_fails() {
    let mut c = campaign(TargetingRecord::default());
    // Creative is 300x250, impression wants 728x90
    let mut i = imp();
    if let Some(b) = i.banner.as_mut() {
        b.format = Some(vec![Format { w: 728, h: 90 }]);
        b.w = Some(728);
        b.h = Some(90);
    }
    assert!(!matches(&c, &basic_req(), &i));
}

#[test]
fn combined_gcc_targeting_matches() {
    let c = campaign(TargetingRecord {
        geo_countries: vec!["ARE".into(), "SAU".into(), "KWT".into()],
        device_types:  vec!["mobile".into()],
        os_types:      vec!["ios".into()],
        languages:     vec!["ar".into(), "en".into()],
        site_categories: vec!["IAB13".into()],
        ..Default::default()
    });
    assert!(matches(&c, &basic_req(), &imp()));
}
