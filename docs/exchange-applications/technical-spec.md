# Technical Integration Spec — GZ Ads DSP

For exchange technical teams evaluating the integration.

---

## Bid endpoint

```
POST https://[YOUR-DOMAIN]/bid
Content-Type: application/json
```

Response: OpenRTB 2.6 BidResponse (JSON) or HTTP 204 No Bid.

**Timeouts:** We respond within 120 ms of receiving the request. If your exchange
requires a shorter timeout (e.g. 80 ms), we can configure our decision loop
accordingly — please advise during onboarding.

---

## Supported OpenRTB fields

### BidRequest (we read)

| Object         | Fields used for targeting                                     |
|----------------|---------------------------------------------------------------|
| `imp[]`        | `id`, `banner`, `native`, `video`, `bidfloor`, `bidfloorcur` |
| `imp.banner`   | `w`, `h`, `format[]`                                         |
| `imp.video`    | `mimes`, `minduration`, `maxduration`, `protocols`           |
| `site`         | `domain`, `cat`, `keywords`, `page`                          |
| `app`          | `bundle`, `domain`, `cat`, `keywords`                        |
| `device`       | `ua`, `ip`, `devicetype`, `os`, `language`, `geo.country`    |
| `user`         | `id`, `buyeruid`, `yob`                                      |

### BidResponse (we send)

```json
{
  "id": "<bid_request_id>",
  "seatbid": [{
    "bid": [{
      "id": "<bid_uuid>",
      "impid": "<imp_id>",
      "price": 2.50,
      "adid": "<creative_id>",
      "nurl": "https://[DOMAIN]/win/${AUCTION_PRICE}",
      "burl": "https://[DOMAIN]/loss",
      "adm":  "<banner markup or VAST XML>",
      "adomain": ["advertiser-domain.com"],
      "crid":  "<creative_id>",
      "w": 300,
      "h": 250
    }],
    "seat": "<seat_id_from_exchange>"
  }],
  "cur": "USD"
}
```

### Win notice (`nurl`)

```
GET https://[DOMAIN]/win/${AUCTION_PRICE}
```

`${AUCTION_PRICE}` is the exchange's price macro. We support the standard OpenRTB
macro `${AUCTION_PRICE}`. If your exchange uses a different macro name, please advise
during onboarding (e.g. Smaato uses `{AUCTION_PRICE}`, AppNexus uses `${AUCTION_PRICE}`).

---

## No-bid response

We return HTTP **204 No Content** when we choose not to bid. This is lower overhead
than a JSON no-bid body — if your exchange requires a JSON no-bid, we can switch.

---

## Currency

All bids are submitted in **USD**. If your exchange settles in a different currency,
please provide the exchange rate mechanism or confirm USD acceptance.

---

## Supported banner sizes

| Size       | Common use            |
|------------|-----------------------|
| 300×250    | Medium rectangle      |
| 320×50     | Mobile banner         |
| 320×100    | Large mobile banner   |
| 728×90     | Leaderboard           |
| 300×600    | Half-page / filmstrip |
| 160×600    | Wide skyscraper       |

---

## IP / user passthrough

We do not modify `device.ip`, `device.ua`, or `user.id` fields. They are passed
through directly to our targeting engine and are not stored beyond the bid decision.

---

## Sandbox / testing

We maintain a staging bidder endpoint available for integration testing:
```
POST https://[STAGING-DOMAIN]/bid
```
We can configure it to always bid at a fixed CPM to confirm win-notice flows.
Please share a set of sample bid requests in your exchange's format so we can
validate field mapping before go-live.

---

## Contact for technical integration

**Email:** jk@istrata.co  
**Company:** Istrata Digital Sdn Bhd (trading as GZ Ads)
