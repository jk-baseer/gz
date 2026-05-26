# DSP Profile — GZ Ads

## Company

| Field              | Detail                                      |
|--------------------|---------------------------------------------|
| Legal name         | Istrata Digital Sdn Bhd                     |
| Trading name       | GZ Ads                                      |
| Country            | Malaysia (Sdn Bhd — private limited)        |
| Contact email      | jk@istrata.co                               |
| Primary market     | GCC / MENA (UAE, Saudi Arabia, Kuwait, Qatar, Bahrain, Oman) |

## Platform overview

GZ Ads is a purpose-built programmatic DSP serving brand and performance advertisers targeting the GCC and broader MENA region. The platform is OpenRTB 2.6 compliant, operates a sub-100 ms decision engine written in Rust, and provides a self-serve campaign management interface.

**Key capabilities**

- OpenRTB 2.6 bid requests (banner, native, video)
- Targeting dimensions: geo (country), device type, OS, language, IAB content categories, domain allowlist/blocklist, keyword contextual, age range (from user.yob)
- Real-time budget pacing and daily frequency capping via Redis
- Win notification and impression/click/conversion tracking
- Self-serve advertiser dashboard with AI-assisted campaign setup

**Advertiser focus**

- Direct brands in retail, finance, automotive, real estate, and technology
- Performance agencies running GCC campaigns
- B2B advertisers targeting Gulf-region business audiences

## Integration

| Parameter              | Value                                              |
|------------------------|----------------------------------------------------|
| Protocol               | OpenRTB 2.6                                        |
| Transport              | HTTPS POST, JSON                                   |
| Bid endpoint           | `https://[YOUR-DOMAIN]/bid`                        |
| QPS capacity           | 5,000 bid requests/second (horizontally scalable)  |
| Bid timeout            | 120 ms (can adjust to exchange requirement)        |
| Win notice             | Supported via `nurl` and `burl`                    |
| Loss notice            | Supported                                          |
| Supported formats      | Banner, Native, Video                              |
| HTTPS                  | Required (TLS 1.2+)                                |
| IP/UA passthrough      | Honoured for targeting and fraud signals           |
| GDPR / Privacy         | No PII stored; user signals passed through as-is   |

## Seat / buyer ID

To be assigned by exchange upon approval. We will configure it immediately on our end.

## Traffic preferences

- **Geographies:** GCC primary (ARE, SAU, KWT, QAT, BHR, OMN); MENA secondary
- **Device:** Mobile, desktop, tablet
- **Formats:** Banner (300×250, 320×50, 728×90, 300×600), Native, Pre-roll video
- **No interest in:** Adult, gambling, incentivised traffic
- **Minimum floor:** Willing to bid on inventory with floors up to $5 CPM

## Commercial

- Payment terms: Net-30 or prepay, whichever exchange requires
- Billing currency: USD
- Credit reference available on request
