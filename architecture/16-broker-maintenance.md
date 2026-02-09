## 16. Broker Database Maintenance

The broker database is the lifeblood of the project. It must be community-maintained and version-controlled.

### 16.1 Update Strategy

1. **Git-based broker definitions** — stored in the `brokers/` directory, versioned with the project
2. **Community contributions** — PRs for new brokers, updated procedures, flagged broken ones
3. **Automated verification** — CI pipeline that periodically tests broker definitions against live sites (checks URLs are reachable, forms exist at expected selectors)
4. **LLM-assisted updates** — when a scan fails, the LLM can attempt to navigate the broker site and propose an updated definition
5. **Versioned definitions** — each broker def has a `last_verified` date; the app warns when definitions are stale

### 16.2 Initial Broker Coverage Target

Focus on the highest-impact brokers first (this is a commonly cited list across the commercial services):

- **Tier 1 (launch):** Spokeo, BeenVerified, WhitePages, FastPeopleSearch, TruePeopleSearch, Intelius, PeopleFinder, Radaris, USSearch, MyLife
- **Tier 2 (v0.2):** Acxiom, Oracle/AddThis, TowerData, Epsilon, LexisNexis (consumer), Pipl, ZabaSearch, AnyWho, Addresses.com, PeopleSmart
- **Tier 3 (v0.3+):** 100+ additional brokers, background check sites, marketing list providers

### 16.3 Domain Intelligence Maintenance

The `domains/` directory contains categorized domain definitions used by the Network Telemetry Engine:

**Seeded from community-maintained lists:**
- EasyList / EasyPrivacy
- Disconnect.me tracking protection lists
- OISD domain blocklist
- Steven Black unified hosts
- California & Vermont data broker registries

**Categories:** DataBroker, AdNetwork, Tracker, Analytics, Fingerprinting, SocialMediaTracker, EmailTracker, RetargetingPlatform

**Format:** TOML definitions per domain with category, tags, opt-out URLs, and verification status.

**Matching:** Trie-based for fast subdomain lookups against observed network connections.

---
