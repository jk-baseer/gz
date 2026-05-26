# Xandr (Microsoft Advertising) — Partnership Email

**To:** Use the partner form at https://www.xandr.com/contact/  
*(Or: xandr-partnerships@microsoft.com — Xandr is now part of Microsoft Advertising)*  
**From:** jk@istrata.co  
**Subject:** DSP Seat Request — GZ Ads / Istrata Digital Sdn Bhd

---

Dear Xandr / Microsoft Advertising Partnerships Team,

I am writing on behalf of **Istrata Digital Sdn Bhd**, operators of **GZ Ads**, a programmatic DSP focused on GCC and MENA advertising. We are applying for a DSP seat on the Xandr marketplace.

**Company overview**

- **Legal entity:** Istrata Digital Sdn Bhd (Malaysia)
- **Brand:** GZ Ads
- **Contact:** jk@istrata.co
- **Market focus:** GCC (UAE, Saudi Arabia, Kuwait, Qatar, Bahrain, Oman) and broader MENA

**Platform capabilities**

GZ Ads is an OpenRTB 2.6 compliant DSP with a Rust-based bidder achieving sub-100 ms decisions at scale. We support banner, native, and video formats, with advanced targeting across geo, device type, IAB content categories, domain lists, keyword contextual signals, and age range.

Our advertiser base consists of direct brands and agencies running performance and brand campaigns targeting Gulf-region audiences — a segment where Xandr's premium publisher relationships are particularly valuable.

**Technical parameters**

| Parameter            | Value                                    |
|----------------------|------------------------------------------|
| Protocol             | OpenRTB 2.6                              |
| Bid endpoint         | `https://[YOUR-DOMAIN]/bid`              |
| Bid timeout          | 120 ms (flexible)                        |
| No-bid response      | HTTP 204 No Content                      |
| Win notice           | `nurl` in bid response                   |
| Price macro          | `${AUCTION_PRICE}`                       |
| Supported formats    | Banner, Native, Video (VAST)             |
| QPS capacity         | 5,000+ (scalable)                        |
| Currency             | USD                                      |
| Monthly spend target | $20,000–$50,000 (growing)                |

**Requested next steps**

1. DSP seat ID assignment
2. Bid stream endpoint and any Xandr-specific technical requirements
3. Standard DSP agreement / IO to review and sign

I am happy to provide company registration documents, a technical integration walkthrough, or a call with your solutions engineering team. Please advise on your preferred onboarding process.

Best regards,  
*(Your name)*  
Istrata Digital Sdn Bhd  
jk@istrata.co

---

*Attachments to include when sending:*
- `dsp-profile.md` (converted to PDF)
- `technical-spec.md` (converted to PDF)
- Company registration certificate (Sdn Bhd)

**Note:** Xandr has a more rigorous vetting process than the others. Expect a longer timeline (2–6 weeks). Submit this while already live on Smaato/Menadex so you can reference active spend when they ask about volume.
