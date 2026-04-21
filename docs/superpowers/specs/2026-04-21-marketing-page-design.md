# Marketing Page Design Spec

**Date:** 2026-04-21
**Status:** Approved

## Overview

A marketing placeholder page for Spectral — a free, open-source Tauri desktop app that scans 130+ data brokers and automates removal requests. The page showcases the tool and provides a secondary download CTA.

---

## Decisions

### Hosting & Stack
- **Host:** Vercel (Hobby plan — free, personal use, no time limit)
- **Framework:** Astro (static output, component-based, easy to grow into docs/blog later)
- **Icons:** Lucide (MIT, tree-shakeable, CDN or npm package)
- **No emojis** — use Lucide icons throughout

### Audience & Tone
- **Dual audience:** general public and tech-savvy users
- **Tone:** friendly + activist — approachable but principled ("your data, your rules")

### Brand
- **Colors:** Light/emerald green palette
  - Primary accent: `#059669`
  - Light green bg: `#f0fdf4`, `#ecfdf5`
  - Border/muted: `#d1fae5`, `#a7f3d0`
  - Dark section: `#111827`
- **Logo:** Ghost in a rounded-square tile (emerald green `#059669`)
  - Ghost is narrower than the tile, positioned left-of-center
  - Left wall and bottom-left lobe overflow the tile edges (depth effect)
  - Smooth circular dome top
  - Asymmetric three-lobe cascading bottom (left lobe droops lowest, below tile bottom)
  - Two eyes: left eye open (circle, higher), right eye winking (arc, lower)
  - Ghost fill: `#ecfdf5` with drop shadow
- **Wordmark:** `spectral` lowercase, `font-weight: 800`, `letter-spacing: -0.5px`

---

## Page Sections (top to bottom)

### 1. Nav
- Logo (large) + wordmark left
- Links: Features, How it works, GitHub
- Primary CTA: Download button (right)
- Sticky, white background, green bottom border

### 2. Hero
- Badge: "Free & Open Source" (pill)
- Headline: **"Data brokers are selling your information."**
- Animated line: **"Disappear with Spectral."**
  - Starts visible, fades out on first mousemove (2.5s transition)
  - After fully faded: waits 3 seconds, then fades back in (2.5s)
  - Cycle repeats indefinitely
- Subtext: platform description
- CTAs: "Download for Free" (primary) + "View on GitHub" (outline)
- Footer note: available platforms, no account required

### 3. Features — "Everything you need to disappear"
9 cards in a 3×3 grid. Each card has:
- White background, green left border (3px), drop shadow
- On hover: card content fades out; ghost logo rises from card bottom to center (spring easing, 0.5s, delayed 0.2s after content fade starts)
- On mouse leave: ghost sinks back, card content fades in

**Cards:**
| Title | Icon | Description |
|---|---|---|
| Broker Scanning | `scan-search` | Searches 130+ data broker sites for your name, address, and other personal details |
| Automated Removal | `mail-x` | Sends removal emails on your behalf, with follow-up reminders |
| AI-Assisted | `brain-circuit` | Uses local or cloud LLMs to handle complex removal flows |
| Encrypted Vault | `lock-keyhole` | All profile data stored in an encrypted local database |
| PII Filtering | `filter-x` | When using cloud AI, sensitive data is tokenized before it leaves Spectral |
| Follow-up Tracking | `bell-ring` | Tracks which brokers haven't responded and resurfaces them automatically |
| Cookie Cleanup | `cookie` | Scans Chrome, Firefox, Edge, Brave, and Safari for tracking cookies tied to data brokers |
| Local PII Scanning | `scan-text` | Scans your device for files containing personal information |
| Adtech Removal | `target` | Sends data deletion requests to 60+ ad networks and marketing platforms |

> **Copy accuracy notes:**
> - "130+" is the current broker count — update as definitions are added
> - PII filtering applies only to Spectral's own LLM calls, not browser automation
> - Adtech removal sends deletion requests (not opt-outs — that would imply only stopping future collection)
> - Cookie cleanup uses `spectral-cookies` crate (scanner, matcher, remover)
> - Local PII scanning uses `spectral-discovery` crate

### 4. How it Works — "Three steps to disappear"
Horizontal step flow (numbered circles, connecting line):
1. Enter your profile — stored encrypted on device
2. Run a scan — Spectral searches every broker
3. Sit back — removal requests go out, follow-ups tracked

### 5. Privacy — "You control what leaves your machine"
Two-column layout:
- Left: checklist of privacy guarantees
- Right: privacy level panel (Paranoid → Local → Balanced → Custom) shown as labelled progress bars

### 6. Open Source — "Built in the open" (dark section)
3-card grid on `#111827`:
- MIT Licensed
- Contributions welcome
- Built with Rust + Svelte

### 7. CTA Banner
- Headline: "Ready to disappear from the internet?"
- Subtext + Download button

### 8. Footer
- Copyright + GitHub + License links

---

## Astro Project Structure

```
spectral-site/           # separate directory or subdirectory
├── src/
│   ├── pages/
│   │   └── index.astro
│   ├── components/
│   │   ├── Nav.astro
│   │   ├── Hero.astro
│   │   ├── Features.astro
│   │   ├── FeatureCard.astro
│   │   ├── HowItWorks.astro
│   │   ├── Privacy.astro
│   │   ├── OpenSource.astro
│   │   ├── CtaBanner.astro
│   │   ├── Footer.astro
│   │   └── Logo.astro      ← shared SVG logo component
│   └── styles/
│       └── global.css
├── public/
│   └── favicon.svg         ← ghost tile as favicon
├── astro.config.mjs
└── package.json
```

The ghost logo SVG should live in `Logo.astro` and be imported everywhere (nav, favicon, feature cards).

## Interactions (client-side JS, minimal)

- **"Disappear with Spectral" animation:** state machine (visible → fading → hidden → reappearing), driven by `mousemove` on `document`
- **Feature card ghost reveal:** pure CSS hover (no JS needed) — `opacity` + `transform` transitions on `.card-content` and `.card-ghost`

## Out of Scope

- Analytics (can add Vercel Analytics later)
- Blog or docs pages
- Contact form or waitlist
- i18n
