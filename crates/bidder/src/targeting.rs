use chrono::Utc;
use openrtb::{BidRequest, Imp};

use crate::index::CampaignRecord;

/// Returns true if the campaign should bid on this impression.
pub fn matches(campaign: &CampaignRecord, req: &BidRequest, imp: &Imp) -> bool {
    let now = Utc::now();

    // Date range
    if let Some(start) = campaign.start_date {
        if now < start {
            return false;
        }
    }
    if let Some(end) = campaign.end_date {
        if now > end {
            return false;
        }
    }

    // Dayparting — hour of day (UTC; exchange sends UTC)
    if !campaign.targeting.hours_of_day.is_empty() {
        use chrono::Timelike;
        let hour = now.hour() as i32;
        if !campaign.targeting.hours_of_day.contains(&hour) {
            return false;
        }
    }

    // Dayparting — day of week
    if !campaign.targeting.days_of_week.is_empty() {
        use chrono::Datelike;
        let dow = now.weekday().num_days_from_sunday() as i32;
        if !campaign.targeting.days_of_week.contains(&dow) {
            return false;
        }
    }

    // Bid floor — our CPM must meet the floor
    if let Some(floor) = imp.bidfloor {
        let our_cpm = campaign.bid_price_cpm_cents as f64 / 100.0;
        if our_cpm < floor {
            return false;
        }
    }

    // Geo — country (ISO-3166-1-alpha-3 from OpenRTB)
    if !campaign.targeting.geo_countries.is_empty() {
        let country = req
            .device
            .as_ref()
            .and_then(|d| d.geo.as_ref())
            .and_then(|g| g.country.as_deref());

        match country {
            Some(c) => {
                if !campaign
                    .targeting
                    .geo_countries
                    .iter()
                    .any(|gc| gc.eq_ignore_ascii_case(c))
                {
                    return false;
                }
            }
            None => return false,
        }
    }

    // Device type
    if !campaign.targeting.device_types.is_empty() {
        if let Some(device) = &req.device {
            let dt = device_type_str(device.devicetype.unwrap_or(0));
            if !campaign.targeting.device_types.contains(&dt) {
                return false;
            }
        }
    }

    // OS
    if !campaign.targeting.os_types.is_empty() {
        let os = req.device.as_ref().and_then(|d| d.os.as_deref());
        match os {
            Some(os_str) => {
                if !campaign
                    .targeting
                    .os_types
                    .iter()
                    .any(|o| os_str.to_lowercase().contains(o.as_str()))
                {
                    return false;
                }
            }
            None => return false,
        }
    }

    // Language
    if !campaign.targeting.languages.is_empty() {
        let lang = req.device.as_ref().and_then(|d| d.language.as_deref());
        match lang {
            Some(l) => {
                if !campaign
                    .targeting
                    .languages
                    .iter()
                    .any(|tl| tl.eq_ignore_ascii_case(l))
                {
                    return false;
                }
            }
            None => {} // no language info — don't block
        }
    }

    // IAB content categories
    if !campaign.targeting.site_categories.is_empty() {
        let cats = req
            .site
            .as_ref()
            .and_then(|s| s.cat.as_ref())
            .or_else(|| req.app.as_ref().and_then(|a| a.cat.as_ref()));

        if let Some(page_cats) = cats {
            let has_match = campaign
                .targeting
                .site_categories
                .iter()
                .any(|tc| page_cats.contains(tc));
            if !has_match {
                return false;
            }
        }
        // If no categories in request, don't block — some publishers don't send them
    }

    // Creative size — must have at least one approved creative matching the imp
    if let Some(banner) = &imp.banner {
        let has_size = campaign.creatives.iter().filter(|c| c.format == "banner").any(|c| {
            let (cw, ch) = match (c.width, c.height) {
                (Some(w), Some(h)) => (w as u32, h as u32),
                _ => return true, // no size constraint on creative
            };

            if let Some(formats) = &banner.format {
                formats.iter().any(|f| f.w == cw && f.h == ch)
            } else {
                let bw_ok = banner.w.map_or(true, |bw| bw == cw);
                let bh_ok = banner.h.map_or(true, |bh| bh == ch);
                bw_ok && bh_ok
            }
        });

        if !has_size {
            return false;
        }
    }

    true
}

pub fn device_type_str(dt: u32) -> String {
    match dt {
        4 | 1 => "mobile".to_string(),
        5     => "tablet".to_string(),
        2     => "desktop".to_string(),
        3 | 6 | 7 => "connected_tv".to_string(),
        _     => "unknown".to_string(),
    }
}

pub fn pick_creative<'a>(
    campaign: &'a CampaignRecord,
    imp: &Imp,
) -> Option<&'a crate::index::CreativeRecord> {
    if imp.banner.is_some() {
        campaign
            .creatives
            .iter()
            .filter(|c| c.format == "banner")
            .find(|c| {
                if let Some(banner) = &imp.banner {
                    let (cw, ch) = match (c.width, c.height) {
                        (Some(w), Some(h)) => (w as u32, h as u32),
                        _ => return true,
                    };
                    if let Some(formats) = &banner.format {
                        formats.iter().any(|f| f.w == cw && f.h == ch)
                    } else {
                        banner.w.map_or(true, |bw| bw == cw)
                            && banner.h.map_or(true, |bh| bh == ch)
                    }
                } else {
                    false
                }
            })
    } else {
        None
    }
}
