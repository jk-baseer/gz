# Smaato — DSP Onboarding Portal Answers

Self-serve signup at: https://wiki.smaato.com/display/DSP/DSP+Onboarding+Guide
Portal login / account creation: https://www.smaato.com/demand/

Fill in the portal form using the answers below.

---

## Company information

| Field                  | Answer                              |
|------------------------|-------------------------------------|
| Company name (legal)   | Istrata Digital Sdn Bhd             |
| Trading / brand name   | GZ Ads                              |
| Company website        | https://istrata.co                  |
| Country of registration | Malaysia                           |
| Primary contact name   | *(your name)*                       |
| Primary contact email  | jk@istrata.co                       |
| Phone                  | *(your number)*                     |

## DSP technical details

| Field                     | Answer                                          |
|---------------------------|-------------------------------------------------|
| Bid endpoint URL          | `https://[YOUR-DOMAIN]/bid`                     |
| OpenRTB version           | 2.6                                             |
| QPS                       | 5,000 (scalable)                                |
| Bid timeout (ms)          | 120 (adjustable)                                |
| No-bid response           | HTTP 204                                        |
| Win notice support        | Yes — `nurl` field in bid response              |
| Price macro               | `{AUCTION_PRICE}` (Smaato format)               |
| Supported formats         | Banner, Native, Video                           |
| Supported banner sizes    | 300×250, 320×50, 320×100, 728×90, 300×600       |
| Payment method preference | Wire transfer / prepay                          |
| Monthly budget estimate   | $2,000–$5,000 (scaling)                         |

## Traffic preferences

| Field               | Answer                                               |
|---------------------|------------------------------------------------------|
| Target geographies  | UAE, Saudi Arabia, Kuwait, Qatar, Bahrain, Oman      |
| Device types        | Mobile, desktop, tablet                              |
| Content categories  | Business, Finance, Tech, Automotive, Real Estate     |
| Blocked categories  | Adult, gambling, alcohol                             |
| Minimum floor bid   | Will bid up to $5 CPM                                |

## Notes for Smaato portal

- If asked for a "seat ID" leave blank — Smaato assigns this on approval.
- Use the sandbox endpoint they provide to run test bid requests before going live.
- In the "Integration Type" dropdown select **Real-Time Bidding (RTB)** → **OpenRTB**.
- If asked for sample bid request JSON, paste the example from `../technical-spec.md`.
