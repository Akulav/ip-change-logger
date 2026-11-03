# Data sources

IPTrail 2.x relied on [ipdata.co](https://ipdata.co), which needs a personal API
key. IPTrail 3 rebuilds the same picture from free, keyless sources.

| IPLogger 2.x (ipdata.co) | IPTrail 3 | Source |
|---|---|---|
| Time zone / location | Location, region, country, time zone, coordinates | ip-api.com → ipwho.is |
| — | ISP, organization, ASN, reverse DNS | ip-api.com → ipwho.is |
| Is proxy | **Proxy / VPN** | ip-api.com `proxy` |
| Is Tor | **Tor exit node** | Tor Project bulk exit list |
| — | **Data center** | ip-api.com `hosting` |
| — | **Mobile network** | ip-api.com `mobile` |
| Is known attacker | **Known attacker** | blocklist.de (`all.txt`) |
| Is known abuser | **Known abuser** | FireHOL level 1 |
| Is threat | **Threat** = attacker or abuser | derived |
| Is anonymous | **Anonymous** = Tor or proxy | derived |

## Public IP providers

Plain-text, HTTPS, IPv4 echo services, tried in this order until one answers
with a valid address:

1. `https://api.ipify.org`
2. `https://ipv4.icanhazip.com`
3. `https://checkip.amazonaws.com`
4. `https://v4.ident.me`

## ip-api.com

Location, network and the `proxy`, `hosting` and `mobile` flags.

- The free endpoint is **HTTP only** — the request (which contains your public IP)
  is not encrypted. If that matters to you, turn off *Look up location, network and
  proxy/VPN status* in Settings; you will still get the Tor and blocklist checks.
- Limit: 45 requests per minute. IPTrail only asks when the address changes.
- Free use is for non-commercial purposes; see ip-api.com's terms.

If ip-api.com fails, IPTrail falls back to **ipwho.is** over HTTPS. That gives
location and network but no proxy/hosting/mobile flags (shown as *Unknown*).

## Threat feeds

Downloaded only when needed, cached in the cache directory, and refreshed after:

| Feed | URL | Refresh | Approx. size |
|---|---|---|---|
| Tor exits | `https://check.torproject.org/torbulkexitlist` | 1 h | ~20 KB |
| blocklist.de | `https://lists.blocklist.de/lists/all.txt` | 6 h | ~350 KB |
| FireHOL level 1 | `https://raw.githubusercontent.com/firehol/blocklist-ipsets/master/firehol_level1.netset` | 12 h | ~75 KB |

If a refresh fails, the previous copy keeps being used and a warning appears in
the footer. Matching happens locally — the feeds never learn your address.

## Limits worth knowing

- Proxy/VPN detection is heuristic. Small or private VPN servers are often not
  flagged; residential proxies almost never are.
- Blocklist hits on a *shared* address (CGNAT, mobile carriers, popular VPN exits)
  usually mean someone else on that address misbehaved, not you.
- IPv6: IPTrail tracks your public **IPv4** address. Blocklist CIDR matching is
  IPv4-only.
