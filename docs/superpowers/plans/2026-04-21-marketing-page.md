# Marketing Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and deploy a one-page Astro marketing site for Spectral to Vercel.

**Architecture:** Astro static site in `site/` inside the monorepo. Each page section is a scoped Astro component with its own `<style>` block. Client-side interactivity (hero fade animation, card ghost reveal) uses vanilla JS and CSS — no client-side framework. Vercel deploys from the `site/` root directory.

**Tech Stack:** Astro 5 (static), Lucide (npm, tree-shaken), vanilla CSS with custom properties, Vercel

---

## File Map

| File | Responsibility |
|---|---|
| `site/astro.config.mjs` | Astro config (static output) |
| `site/src/layouts/Layout.astro` | HTML shell, meta, Lucide init script |
| `site/src/styles/global.css` | CSS reset, custom properties, utility classes |
| `site/src/components/Logo.astro` | Ghost SVG — shared by Nav, FeatureCard, favicon |
| `site/public/favicon.svg` | Simplified ghost tile for browser tab |
| `site/src/components/Nav.astro` | Sticky top nav |
| `site/src/components/Hero.astro` | Hero section + disappear animation |
| `site/src/components/FeatureCard.astro` | Single feature card with ghost hover reveal |
| `site/src/components/Features.astro` | 3×3 grid of FeatureCards |
| `site/src/components/HowItWorks.astro` | 3-step horizontal flow |
| `site/src/components/Privacy.astro` | Checklist + privacy level panel |
| `site/src/components/OpenSource.astro` | Dark section, 3-card grid |
| `site/src/components/CtaBanner.astro` | Final download CTA |
| `site/src/components/Footer.astro` | Copyright + links |
| `site/src/pages/index.astro` | Page assembly |
| `site/vercel.json` | Vercel deployment config |

---

## Task 1: Scaffold the Astro project

**Files:**
- Create: `site/` (entire scaffolded project)
- Modify: `.gitignore` (root)

- [ ] **Step 1: Scaffold Astro in `site/`**

```bash
cd /home/evan/projects/spectral
npm create astro@latest site -- --template minimal --no-git --install --skip-houston
```

When prompted for TypeScript, select **Strict**. When asked about Git, say No (the root repo handles that).

Expected output ends with: `✔ Project initialised!`

- [ ] **Step 2: Install Lucide**

```bash
cd /home/evan/projects/spectral/site
npm install lucide
```

- [ ] **Step 3: Add `site/` to root `.gitignore`**

Append to `/home/evan/projects/spectral/.gitignore`:
```
site/node_modules/
site/dist/
site/.astro/
```

- [ ] **Step 4: Verify dev server starts**

```bash
cd /home/evan/projects/spectral/site
npm run dev
```

Expected: server running at `http://localhost:4321` with default Astro page.

- [ ] **Step 5: Commit**

```bash
cd /home/evan/projects/spectral
git add site/ .gitignore
git commit -m "feat(site): scaffold Astro marketing site"
```

---

## Task 2: Layout + global styles

**Files:**
- Create: `site/src/layouts/Layout.astro`
- Create: `site/src/styles/global.css`
- Modify: `site/astro.config.mjs`

- [ ] **Step 1: Configure static output**

Replace `site/astro.config.mjs` with:
```js
import { defineConfig } from 'astro/config';

export default defineConfig({
  output: 'static',
});
```

- [ ] **Step 2: Create global CSS**

Create `site/src/styles/global.css`:
```css
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --green-50:  #ecfdf5;
  --green-100: #d1fae5;
  --green-200: #a7f3d0;
  --green-300: #6ee7b7;
  --green-500: #059669;
  --green-700: #047857;
  --green-900: #065f46;
  --green-bg:  #f0fdf4;
  --dark:      #111827;
  --gray-100:  #f9fafb;
  --gray-200:  #e5e7eb;
  --gray-400:  #9ca3af;
  --gray-500:  #6b7280;
  --gray-600:  #4b5563;
  --gray-700:  #374151;
  --text:      #111827;

  --font: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
}

html { font-family: var(--font); background: var(--green-bg); color: var(--text); }

a { text-decoration: none; }

.btn-primary {
  display: inline-flex; align-items: center; gap: 6px;
  background: var(--green-500); color: #fff;
  padding: 8px 18px; border-radius: var(--radius-sm);
  font-size: 14px; font-weight: 600; border: none; cursor: pointer;
  transition: background 0.15s;
}
.btn-primary:hover { background: var(--green-700); }

.btn-outline {
  display: inline-flex; align-items: center; gap: 6px;
  background: transparent; color: var(--green-500);
  padding: 8px 18px; border-radius: var(--radius-sm);
  font-size: 14px; font-weight: 600;
  border: 1.5px solid var(--green-500); cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.btn-outline:hover { background: var(--green-50); }

.section-label {
  text-align: center; font-size: 12px; font-weight: 700;
  color: var(--green-500); letter-spacing: 2px;
  text-transform: uppercase; margin-bottom: 10px;
}
.section-title {
  text-align: center; font-size: 28px; font-weight: 800;
  color: var(--text); letter-spacing: -0.5px; margin-bottom: 8px;
}
.section-sub {
  text-align: center; font-size: 15px; color: var(--gray-500);
  max-width: 480px; margin: 0 auto 40px; line-height: 1.6;
}
```

- [ ] **Step 3: Create Layout.astro**

Create `site/src/layouts/Layout.astro`:
```astro
---
interface Props {
  title?: string;
  description?: string;
}
const {
  title = 'Spectral — Disappear from the internet',
  description = 'Free, open-source desktop app that scans 130+ data brokers and automates removal requests.',
} = Astro.props;
---
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description" content={description} />
    <title>{title}</title>
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
  </head>
  <body>
    <slot />
  </body>
</html>

<style is:global>
  @import '../styles/global.css';
</style>

<script>
  import {
    createIcons,
    ScanSearch, MailX, BrainCircuit, LockKeyhole, FilterX,
    BellRing, Cookie, ScanText, Target, ShieldCheck, Download,
    Github, CheckCircle2, Code2, GitPullRequest, Package,
  } from 'lucide';

  createIcons({
    icons: {
      ScanSearch, MailX, BrainCircuit, LockKeyhole, FilterX,
      BellRing, Cookie, ScanText, Target, ShieldCheck, Download,
      Github, CheckCircle2, Code2, GitPullRequest, Package,
    },
  });
</script>
```

- [ ] **Step 4: Verify check passes**

```bash
cd /home/evan/projects/spectral/site
npx astro check
```

Expected: `Found 0 errors`

- [ ] **Step 5: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add layout and global styles"
```

---

## Task 3: Logo component + favicon

**Files:**
- Create: `site/src/components/Logo.astro`
- Create: `site/public/favicon.svg`

- [ ] **Step 1: Create Logo.astro**

Create `site/src/components/Logo.astro`:
```astro
---
interface Props {
  width?: number;
  height?: number;
}
const { width = 72, height = 80 } = Astro.props;
---
<svg
  width={width}
  height={height}
  viewBox="0 0 80 96"
  style="overflow:visible;"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
>
  <defs>
    <filter id="ghost-shadow" x="-20%" y="-10%" width="180%" height="160%">
      <feDropShadow dx="2" dy="4" stdDeviation="4" flood-color="#065f46" flood-opacity="0.22" />
    </filter>
  </defs>
  <!-- Green tile -->
  <rect x="12" y="4" width="64" height="64" rx="16" fill="#059669" />
  <!-- Ghost body: left wall and bottom-left lobe overflow tile edges -->
  <path
    d="M 27 8 C 40 8 50 18 50 31 L 50 56
       C 50 59 48 61 46 62 C 44 63 43 65 42 66
       C 40 68 38 67 36 65 C 34 63 33 67 31 71
       C 29 76 27 84 24 84 C 21 84 19 76 17 71
       C 15 66 15 61 13 57 C 10 53 7 52 4 55
       L 4 31 C 4 18 14 8 27 8 Z"
    fill="#ecfdf5"
    filter="url(#ghost-shadow)"
  />
  <!-- Left eye: open, higher -->
  <circle cx="19" cy="28" r="3" fill="#059669" />
  <!-- Right eye: winking -->
  <path
    d="M 33 34 Q 38 30 43 34"
    stroke="#059669"
    stroke-width="2.5"
    stroke-linecap="round"
    fill="none"
  />
</svg>
```

- [ ] **Step 2: Create favicon.svg**

Create `site/public/favicon.svg`:
```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="12 4 64 64" width="64" height="64">
  <rect x="12" y="4" width="64" height="64" rx="16" fill="#059669"/>
  <path
    d="M 27 8 C 40 8 50 18 50 31 L 50 56
       C 50 59 48 61 46 62 C 44 63 43 65 42 66
       C 40 68 38 67 36 65 C 34 63 33 67 31 71
       C 29 76 27 84 24 84 C 21 84 19 76 17 71
       C 15 66 15 61 13 57 C 10 53 7 52 4 55
       L 4 31 C 4 18 14 8 27 8 Z"
    fill="#ecfdf5"
  />
  <circle cx="19" cy="28" r="3" fill="#059669"/>
  <path d="M 33 34 Q 38 30 43 34" stroke="#059669" stroke-width="2.5" stroke-linecap="round" fill="none"/>
</svg>
```

Note: the favicon viewBox is cropped to just the tile (`12 4 64 64`) so the ghost overflow is clipped — this gives a clean square icon.

- [ ] **Step 3: Smoke-test Logo in index.astro**

Temporarily replace `site/src/pages/index.astro` with:
```astro
---
import Layout from '../layouts/Layout.astro';
import Logo from '../components/Logo.astro';
---
<Layout>
  <div style="padding:40px; background:#f0fdf4; display:flex; gap:20px; align-items:center;">
    <Logo width={80} height={90} />
    <Logo width={48} height={54} />
    <Logo width={32} height={36} />
    <span style="font-size:24px;font-weight:800;color:#059669;">spectral</span>
  </div>
</Layout>
```

Open `http://localhost:4321` and verify the ghost logo renders at all three sizes. The ghost should overflow the tile bottom-left.

- [ ] **Step 4: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add Logo component and favicon"
```

---

## Task 4: Nav component

**Files:**
- Create: `site/src/components/Nav.astro`

- [ ] **Step 1: Create Nav.astro**

Create `site/src/components/Nav.astro`:
```astro
---
import Logo from './Logo.astro';
---
<nav>
  <a href="/" class="nav-logo">
    <Logo width={52} height={58} />
    <span>spectral</span>
  </a>
  <div class="nav-links">
    <a href="#features">Features</a>
    <a href="#how-it-works">How it works</a>
    <a href="https://github.com/InfiniteInsight/spectral" target="_blank" rel="noopener">GitHub</a>
    <a
      href="https://github.com/InfiniteInsight/spectral/releases/latest"
      class="btn-primary"
      target="_blank"
      rel="noopener"
    >
      <i data-lucide="download" style="width:14px;height:14px;"></i>
      Download
    </a>
  </div>
</nav>

<style>
  nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 40px;
    background: #fff;
    border-bottom: 1px solid var(--green-100);
    position: sticky;
    top: 0;
    z-index: 100;
  }

  .nav-logo {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 20px;
    font-weight: 800;
    color: var(--green-500);
    letter-spacing: -0.5px;
  }

  .nav-links {
    display: flex;
    gap: 24px;
    align-items: center;
  }

  .nav-links a {
    font-size: 14px;
    color: var(--gray-700);
    transition: color 0.15s;
  }

  .nav-links a:hover { color: var(--green-500); }
</style>
```

- [ ] **Step 2: Add Nav to index.astro and verify**

Update `site/src/pages/index.astro`:
```astro
---
import Layout from '../layouts/Layout.astro';
import Nav from '../components/Nav.astro';
---
<Layout>
  <Nav />
  <main style="padding:40px;">Page content here</main>
</Layout>
```

Open `http://localhost:4321`. Verify: sticky nav with ghost logo, wordmark, links, and Download button. Lucide download icon should render.

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add Nav component"
```

---

## Task 5: Hero section

**Files:**
- Create: `site/src/components/Hero.astro`

- [ ] **Step 1: Create Hero.astro**

Create `site/src/components/Hero.astro`:
```astro
<section class="hero">
  <div class="badge">
    <i data-lucide="shield-check" style="width:12px;height:12px;"></i>
    Free &amp; Open Source
  </div>
  <h1>
    Data brokers are selling your information.<br />
    <span class="disappear-text">Disappear with Spectral.</span>
  </h1>
  <p>
    Automatically scan 130+ data broker sites, send removal requests, and take back
    control of your personal information — all from your desktop.
  </p>
  <div class="hero-cta">
    <a
      href="https://github.com/InfiniteInsight/spectral/releases/latest"
      class="btn-primary"
      target="_blank"
      rel="noopener"
    >
      <i data-lucide="download" style="width:14px;height:14px;"></i>
      Download for Free
    </a>
    <a
      href="https://github.com/InfiniteInsight/spectral"
      class="btn-outline"
      target="_blank"
      rel="noopener"
    >
      <i data-lucide="github" style="width:14px;height:14px;"></i>
      View on GitHub
    </a>
  </div>
  <p class="hero-note">Available for Windows, macOS, and Linux &nbsp;·&nbsp; No account required</p>
</section>

<style>
  .hero {
    text-align: center;
    padding: 80px 40px 60px;
    background: linear-gradient(160deg, var(--green-50) 0%, var(--green-bg) 60%, #fff 100%);
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--green-100);
    color: var(--green-900);
    font-size: 12px;
    font-weight: 600;
    padding: 4px 12px;
    border-radius: 99px;
    margin-bottom: 20px;
    border: 1px solid var(--green-200);
  }

  h1 {
    font-size: 48px;
    font-weight: 900;
    line-height: 1.1;
    color: var(--text);
    letter-spacing: -1.5px;
    margin-bottom: 16px;
    max-width: 680px;
    margin-left: auto;
    margin-right: auto;
  }

  .disappear-text {
    display: block;
    color: var(--green-500);
    opacity: 1;
    transition: opacity 2.5s ease;
    user-select: none;
  }

  p {
    font-size: 18px;
    color: var(--gray-600);
    max-width: 520px;
    margin: 0 auto 32px;
    line-height: 1.6;
  }

  .hero-cta {
    display: flex;
    gap: 12px;
    justify-content: center;
    flex-wrap: wrap;
  }

  .hero-note {
    font-size: 12px;
    color: var(--gray-400);
    margin-top: 16px;
    margin-bottom: 0;
  }
</style>

<script>
  const text = document.querySelector<HTMLElement>('.disappear-text')!;
  let state: 'visible' | 'fading' | 'hidden' | 'reappearing' = 'visible';

  document.addEventListener('mousemove', () => {
    if (state !== 'visible') return;
    state = 'fading';
    text.style.opacity = '0';
    // wait for 2.5s CSS transition to complete, then dwell 3s, then reappear
    setTimeout(() => {
      state = 'hidden';
      setTimeout(() => {
        state = 'reappearing';
        text.style.opacity = '1';
        setTimeout(() => { state = 'visible'; }, 2500);
      }, 3000);
    }, 2500);
  });
</script>
```

- [ ] **Step 2: Add Hero to index.astro and verify**

Update `site/src/pages/index.astro`:
```astro
---
import Layout from '../layouts/Layout.astro';
import Nav from '../components/Nav.astro';
import Hero from '../components/Hero.astro';
---
<Layout>
  <Nav />
  <main>
    <Hero />
  </main>
</Layout>
```

Open `http://localhost:4321`. Verify:
- Badge, headline, and "Disappear with Spectral." render
- Moving the mouse causes the green line to fade over ~2.5s
- After 3s it fades back in
- Both CTA buttons have Lucide icons

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add Hero section with disappear animation"
```

---

## Task 6: FeatureCard component

**Files:**
- Create: `site/src/components/FeatureCard.astro`

- [ ] **Step 1: Create FeatureCard.astro**

Create `site/src/components/FeatureCard.astro`:
```astro
---
interface Props {
  icon: string;
  title: string;
  description: string;
}
const { icon, title, description } = Astro.props;
---
<div class="feature-card">
  <div class="card-content">
    <div class="feature-icon">
      <i data-lucide={icon} style="width:20px;height:20px;"></i>
    </div>
    <h3>{title}</h3>
    <p>{description}</p>
  </div>
  <div class="card-ghost" aria-hidden="true">
    <!-- Ghost logo — same SVG as Logo.astro, inlined here for isolation -->
    <svg width="60" height="72" viewBox="0 0 80 96" style="overflow:visible;" fill="none" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <filter id="card-ghost-shadow" x="-20%" y="-10%" width="180%" height="160%">
          <feDropShadow dx="2" dy="4" stdDeviation="4" flood-color="#065f46" flood-opacity="0.22" />
        </filter>
      </defs>
      <rect x="12" y="4" width="64" height="64" rx="16" fill="#059669" />
      <path
        d="M 27 8 C 40 8 50 18 50 31 L 50 56
           C 50 59 48 61 46 62 C 44 63 43 65 42 66
           C 40 68 38 67 36 65 C 34 63 33 67 31 71
           C 29 76 27 84 24 84 C 21 84 19 76 17 71
           C 15 66 15 61 13 57 C 10 53 7 52 4 55
           L 4 31 C 4 18 14 8 27 8 Z"
        fill="#ecfdf5"
        filter="url(#card-ghost-shadow)"
      />
      <circle cx="19" cy="28" r="3" fill="#059669" />
      <path d="M 33 34 Q 38 30 43 34" stroke="#059669" stroke-width="2.5" stroke-linecap="round" fill="none" />
    </svg>
  </div>
</div>

<style>
  .feature-card {
    background: #fff;
    border-radius: 14px;
    padding: 24px 24px 24px 20px;
    border-left: 3px solid var(--green-500);
    box-shadow: 0 1px 4px rgba(0,0,0,0.06), 0 4px 16px rgba(5,150,105,0.07);
    transition: box-shadow 0.2s ease, border-color 0.2s ease, transform 0.2s ease;
    position: relative;
    overflow: visible;
    cursor: default;
  }

  .feature-card:hover {
    transform: translateY(-3px);
    box-shadow: 0 8px 32px rgba(5,150,105,0.15), 0 2px 8px rgba(0,0,0,0.08);
    border-color: var(--green-300);
  }

  .card-content {
    position: relative;
    z-index: 1;
    transition: opacity 0.3s ease;
  }

  .feature-card:hover .card-content { opacity: 0; }

  .feature-icon {
    width: 40px;
    height: 40px;
    background: linear-gradient(135deg, var(--green-100), var(--green-200));
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 14px;
    color: var(--green-500);
    box-shadow: 0 2px 6px rgba(5,150,105,0.15);
  }

  h3 {
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 6px;
    color: var(--text);
  }

  p {
    font-size: 13px;
    color: var(--gray-500);
    line-height: 1.55;
  }

  .card-ghost {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    pointer-events: none;
    /* starts near card bottom, floats up to center (+12px compensates for ghost visual weight below its bounding box) */
    transform: translateY(60px) scale(0.85);
    transition: opacity 0.35s ease 0.2s, transform 0.5s cubic-bezier(0.22,1,0.36,1) 0.2s;
  }

  .feature-card:hover .card-ghost {
    opacity: 1;
    transform: translateY(12px) scale(1);
  }
</style>
```

- [ ] **Step 2: Smoke-test the card**

Update `site/src/pages/index.astro`:
```astro
---
import Layout from '../layouts/Layout.astro';
import Nav from '../components/Nav.astro';
import Hero from '../components/Hero.astro';
import FeatureCard from '../components/FeatureCard.astro';
---
<Layout>
  <Nav />
  <main>
    <Hero />
    <div style="padding:40px;display:grid;grid-template-columns:repeat(3,1fr);gap:20px;max-width:960px;margin:0 auto;">
      <FeatureCard icon="scan-search" title="Broker Scanning" description="Searches 130+ data broker sites for your name, address, and other personal details." />
      <FeatureCard icon="mail-x" title="Automated Removal" description="Sends removal emails on your behalf, with follow-up reminders." />
      <FeatureCard icon="brain-circuit" title="AI-Assisted" description="Uses local or cloud LLMs to handle complex removal flows." />
    </div>
  </main>
</Layout>
```

Open `http://localhost:4321`. Hover over a card: content should fade, ghost logo should rise from the bottom to center of the card. Move away: ghost sinks, content returns.

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add FeatureCard with ghost hover reveal"
```

---

## Task 7: Features section

**Files:**
- Create: `site/src/components/Features.astro`

- [ ] **Step 1: Create Features.astro**

Create `site/src/components/Features.astro`:
```astro
---
import FeatureCard from './FeatureCard.astro';

const cards = [
  { icon: 'scan-search',   title: 'Broker Scanning',     description: 'Searches 130+ data broker sites for your name, address, and other personal details.' },
  { icon: 'mail-x',        title: 'Automated Removal',    description: 'Sends removal emails on your behalf, with follow-up reminders.' },
  { icon: 'brain-circuit', title: 'AI-Assisted',          description: 'Uses local or cloud LLMs to handle complex removal flows — your data stays yours.' },
  { icon: 'lock-keyhole',  title: 'Encrypted Vault',      description: 'All profile data stored in an encrypted local database. Nothing leaves your machine without your consent.' },
  { icon: 'filter-x',      title: 'PII Filtering',        description: 'When using cloud AI, sensitive data is tokenized before it leaves Spectral.' },
  { icon: 'bell-ring',     title: 'Follow-up Tracking',   description: 'Tracks which brokers haven\'t responded and resurfaces them automatically.' },
  { icon: 'cookie',        title: 'Cookie Cleanup',       description: 'Scans Chrome, Firefox, Edge, Brave, and Safari for tracking cookies tied to data brokers and removes them safely.' },
  { icon: 'scan-text',     title: 'Local PII Scanning',   description: 'Scans your device for files containing personal information — names, emails, SSNs — so you know what\'s sitting locally.' },
  { icon: 'target',        title: 'Adtech Removal',       description: 'Sends data deletion requests to 60+ ad networks and marketing platforms — Google Ads, Meta, The Trade Desk, and more.' },
];
---
<section id="features" class="features">
  <p class="section-label">What it does</p>
  <h2 class="section-title">Everything you need to disappear</h2>
  <p class="section-sub">Spectral handles the tedious, repetitive work of data broker removal — so you don't have to.</p>
  <div class="grid">
    {cards.map((card) => (
      <FeatureCard icon={card.icon} title={card.title} description={card.description} />
    ))}
  </div>
</section>

<style>
  .features {
    padding: 72px 40px;
    background: #fff;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 20px;
    max-width: 960px;
    margin: 0 auto;
  }
</style>
```

- [ ] **Step 2: Wire into index.astro and verify**

Update `site/src/pages/index.astro`:
```astro
---
import Layout from '../layouts/Layout.astro';
import Nav from '../components/Nav.astro';
import Hero from '../components/Hero.astro';
import Features from '../components/Features.astro';
---
<Layout>
  <Nav />
  <main>
    <Hero />
    <Features />
  </main>
</Layout>
```

Verify: all 9 cards render, icons appear, hover works on each.

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add Features section"
```

---

## Task 8: How it Works section

**Files:**
- Create: `site/src/components/HowItWorks.astro`

- [ ] **Step 1: Create HowItWorks.astro**

Create `site/src/components/HowItWorks.astro`:
```astro
---
const steps = [
  { n: '1', title: 'Enter your profile',  body: 'Name, address, and a few other details. Stored encrypted on your device.' },
  { n: '2', title: 'Run a scan',          body: 'Spectral searches every broker and finds your listings automatically.' },
  { n: '3', title: 'Sit back',            body: 'Removal requests go out. Follow-ups are tracked. You\'re notified when you\'re gone.' },
];
---
<section id="how-it-works" class="how">
  <p class="section-label">How it works</p>
  <h2 class="section-title">Three steps to disappear</h2>
  <p class="section-sub">No technical knowledge required.</p>
  <div class="steps">
    {steps.map((s) => (
      <div class="step">
        <div class="step-num">{s.n}</div>
        <h4>{s.title}</h4>
        <p>{s.body}</p>
      </div>
    ))}
  </div>
</section>

<style>
  .how {
    padding: 72px 40px;
    background: var(--green-bg);
  }

  .steps {
    display: flex;
    max-width: 800px;
    margin: 0 auto;
    position: relative;
  }

  /* connecting line between steps */
  .steps::before {
    content: '';
    position: absolute;
    top: 20px;
    left: calc(100% / 6);
    right: calc(100% / 6);
    height: 2px;
    background: var(--green-200);
    z-index: 0;
  }

  .step {
    flex: 1;
    text-align: center;
    padding: 0 16px;
    position: relative;
    z-index: 1;
  }

  .step-num {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--green-500);
    color: #fff;
    font-weight: 800;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 auto 12px;
  }

  h4 {
    font-size: 14px;
    font-weight: 700;
    margin-bottom: 4px;
  }

  p {
    font-size: 12px;
    color: var(--gray-500);
    line-height: 1.5;
  }
</style>
```

- [ ] **Step 2: Add to index.astro**

```astro
---
import Layout from '../layouts/Layout.astro';
import Nav from '../components/Nav.astro';
import Hero from '../components/Hero.astro';
import Features from '../components/Features.astro';
import HowItWorks from '../components/HowItWorks.astro';
---
<Layout>
  <Nav />
  <main>
    <Hero />
    <Features />
    <HowItWorks />
  </main>
</Layout>
```

Verify: 3 numbered steps with a connecting line, green background.

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add HowItWorks section"
```

---

## Task 9: Privacy section

**Files:**
- Create: `site/src/components/Privacy.astro`

- [ ] **Step 1: Create Privacy.astro**

Create `site/src/components/Privacy.astro`:
```astro
---
const guarantees = [
  'Run entirely offline with a local LLM',
  'When using cloud AI, sensitive data is tokenized before it leaves Spectral',
  'No account, no telemetry, no subscription',
  'Encrypted local database for all your data',
  'Open source — audit it yourself',
];

const levels = [
  { label: 'Paranoid — fully offline',      pct: 100 },
  { label: 'Local — local LLM only',        pct: 80  },
  { label: 'Balanced — PII-filtered cloud', pct: 55  },
  { label: 'Custom — your choice',          pct: 35  },
];
---
<section class="privacy">
  <p class="section-label">Privacy first</p>
  <h2 class="section-title">You control what leaves your machine</h2>
  <p class="section-sub">Choose your privacy level — from fully local to cloud-assisted.</p>
  <div class="grid">
    <ul class="checklist">
      {guarantees.map((g) => (
        <li>
          <i data-lucide="check-circle-2" style="width:16px;height:16px;flex-shrink:0;color:#059669;margin-top:2px;"></i>
          {g}
        </li>
      ))}
    </ul>
    <div class="panel">
      <h4>Privacy Levels</h4>
      {levels.map((l) => (
        <div class="level">
          <div class="level-label">{l.label}</div>
          <div class="bar"><div class="fill" style={`width:${l.pct}%`}></div></div>
        </div>
      ))}
    </div>
  </div>
</section>

<style>
  .privacy {
    padding: 72px 40px;
    background: #fff;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 40px;
    max-width: 800px;
    margin: 0 auto;
    align-items: center;
  }

  .checklist {
    list-style: none;
  }

  .checklist li {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-size: 14px;
    color: var(--gray-700);
    margin-bottom: 14px;
    line-height: 1.5;
  }

  .panel {
    background: var(--green-bg);
    border: 1px solid var(--green-200);
    border-radius: var(--radius-md);
    padding: 24px;
  }

  h4 {
    font-size: 13px;
    font-weight: 700;
    color: var(--green-900);
    margin-bottom: 16px;
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .level { margin-bottom: 12px; }

  .level-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--gray-700);
    margin-bottom: 4px;
  }

  .bar {
    height: 6px;
    background: var(--green-100);
    border-radius: 3px;
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--green-500);
    border-radius: 3px;
  }
</style>
```

- [ ] **Step 2: Add to index.astro and verify**

Add `import Privacy from '../components/Privacy.astro';` and `<Privacy />` after `<HowItWorks />`.

Verify: two-column layout, checklist icons render, privacy level bars show correct widths.

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add Privacy section"
```

---

## Task 10: Open Source section

**Files:**
- Create: `site/src/components/OpenSource.astro`

- [ ] **Step 1: Create OpenSource.astro**

Create `site/src/components/OpenSource.astro`:
```astro
---
const items = [
  { icon: 'code-2',          title: 'MIT Licensed',           body: 'Free to use, fork, and modify' },
  { icon: 'git-pull-request',title: 'Contributions welcome',  body: 'Open PRs, open issues' },
  { icon: 'package',         title: 'Built with Rust + Svelte', body: 'Fast, safe, cross-platform' },
];
---
<section class="opensource">
  <p class="section-label">Open Source</p>
  <h2 class="section-title">Built in the open</h2>
  <p class="section-sub">No black boxes. Audit the code, contribute, or build on top of it.</p>
  <div class="grid">
    {items.map((item) => (
      <div class="os-card">
        <i data-lucide={item.icon} style={`width:24px;height:24px;color:#6ee7b7;display:block;margin-bottom:8px;`}></i>
        <h4>{item.title}</h4>
        <p>{item.body}</p>
      </div>
    ))}
  </div>
</section>

<style>
  .opensource {
    padding: 72px 40px;
    background: var(--dark);
  }

  .opensource :global(.section-label) { color: #6ee7b7; }
  .section-label { text-align:center; font-size:12px; font-weight:700; color:#6ee7b7; letter-spacing:2px; text-transform:uppercase; margin-bottom:10px; }
  .section-title { text-align:center; font-size:28px; font-weight:800; color:#fff; letter-spacing:-0.5px; margin-bottom:8px; }
  .section-sub   { text-align:center; font-size:15px; color:#9ca3af; max-width:480px; margin:0 auto 40px; line-height:1.6; }

  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    max-width: 700px;
    margin: 0 auto;
  }

  .os-card {
    background: #1f2937;
    border: 1px solid #374151;
    border-radius: 10px;
    padding: 20px;
    text-align: center;
  }

  h4 {
    font-size: 13px;
    font-weight: 700;
    color: #f9fafb;
    margin-bottom: 4px;
  }

  p {
    font-size: 12px;
    color: #9ca3af;
  }
</style>
```

- [ ] **Step 2: Add to index.astro and verify**

Add `import OpenSource from '../components/OpenSource.astro';` and `<OpenSource />` after `<Privacy />`.

Verify: dark section renders, 3 cards with green icons.

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add OpenSource section"
```

---

## Task 11: CTA banner + Footer

**Files:**
- Create: `site/src/components/CtaBanner.astro`
- Create: `site/src/components/Footer.astro`

- [ ] **Step 1: Create CtaBanner.astro**

Create `site/src/components/CtaBanner.astro`:
```astro
<section class="cta-banner">
  <h2>Ready to disappear from the internet?</h2>
  <p>Download Spectral and start your first scan in minutes. It's free, forever.</p>
  <a
    href="https://github.com/InfiniteInsight/spectral/releases/latest"
    class="btn-primary"
    target="_blank"
    rel="noopener"
    style="font-size:15px;padding:10px 24px;"
  >
    <i data-lucide="download" style="width:16px;height:16px;"></i>
    Download Spectral
  </a>
</section>

<style>
  .cta-banner {
    padding: 72px 40px;
    text-align: center;
    background: linear-gradient(135deg, var(--green-50), var(--green-bg));
    border-top: 1px solid var(--green-100);
  }

  h2 {
    font-size: 32px;
    font-weight: 900;
    letter-spacing: -1px;
    margin-bottom: 12px;
  }

  p {
    font-size: 15px;
    color: var(--gray-500);
    margin-bottom: 28px;
  }
</style>
```

- [ ] **Step 2: Create Footer.astro**

Create `site/src/components/Footer.astro`:
```astro
<footer>
  <span>© {new Date().getFullYear()} Spectral · Free &amp; Open Source</span>
  <div class="links">
    <a href="https://github.com/InfiniteInsight/spectral" target="_blank" rel="noopener">GitHub</a>
    <a href="https://github.com/InfiniteInsight/spectral/blob/master/LICENSE" target="_blank" rel="noopener">License</a>
  </div>
</footer>

<style>
  footer {
    padding: 24px 40px;
    background: #fff;
    border-top: 1px solid var(--gray-200);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--gray-400);
  }

  .links {
    display: flex;
    gap: 20px;
  }

  a {
    color: var(--gray-400);
    transition: color 0.15s;
  }

  a:hover { color: var(--green-500); }
</style>
```

- [ ] **Step 3: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): add CtaBanner and Footer"
```

---

## Task 12: Assemble index.astro + verify full build

**Files:**
- Modify: `site/src/pages/index.astro`

- [ ] **Step 1: Replace index.astro with final assembly**

Replace `site/src/pages/index.astro`:
```astro
---
import Layout from '../layouts/Layout.astro';
import Nav from '../components/Nav.astro';
import Hero from '../components/Hero.astro';
import Features from '../components/Features.astro';
import HowItWorks from '../components/HowItWorks.astro';
import Privacy from '../components/Privacy.astro';
import OpenSource from '../components/OpenSource.astro';
import CtaBanner from '../components/CtaBanner.astro';
import Footer from '../components/Footer.astro';
---
<Layout>
  <Nav />
  <main>
    <Hero />
    <Features />
    <HowItWorks />
    <Privacy />
    <OpenSource />
    <CtaBanner />
  </main>
  <Footer />
</Layout>
```

- [ ] **Step 2: Full visual review in dev**

```bash
cd /home/evan/projects/spectral/site
npm run dev
```

Open `http://localhost:4321` and scroll through the full page. Check:
- [ ] Nav sticky, logo visible, links work
- [ ] "Disappear with Spectral." fades on mousemove, returns after 3s
- [ ] All 9 feature cards render with correct icons
- [ ] Card hover: content fades, ghost rises to center, reverses on mouse leave
- [ ] How it works: 3 steps with connecting line
- [ ] Privacy: checklist and progress bars
- [ ] Open source: dark section, 3 green-icon cards
- [ ] CTA banner and footer render

- [ ] **Step 3: Run type check**

```bash
cd /home/evan/projects/spectral/site
npx astro check
```

Expected: `Found 0 errors`

- [ ] **Step 4: Run production build**

```bash
npm run build
```

Expected: no errors, output in `dist/`.

- [ ] **Step 5: Preview production build**

```bash
npm run preview
```

Open `http://localhost:4321` and verify the production build matches dev.

- [ ] **Step 6: Commit**

```bash
cd /home/evan/projects/spectral
git add site/
git commit -m "feat(site): assemble full marketing page"
```

---

## Task 13: Deploy to Vercel

**Files:**
- Create: `site/vercel.json`

- [ ] **Step 1: Create vercel.json**

Create `site/vercel.json`:
```json
{
  "buildCommand": "npm run build",
  "outputDirectory": "dist",
  "installCommand": "npm install"
}
```

- [ ] **Step 2: Push branch to GitHub**

```bash
cd /home/evan/projects/spectral
git push origin feature/broker-enhancements
```

- [ ] **Step 3: Create Vercel project**

1. Go to [vercel.com](https://vercel.com) and sign in
2. Click **Add New → Project**
3. Import the `InfiniteInsight/spectral` repository
4. Under **Root Directory**, set it to `site`
5. Framework Preset will auto-detect **Astro**
6. Click **Deploy**

Expected: Vercel builds successfully and provides a live URL (e.g. `spectral-site.vercel.app`).

- [ ] **Step 4: Set a custom domain (optional)**

In the Vercel project dashboard → Settings → Domains, add your domain (e.g. `spectralapp.com`) and follow the DNS instructions.

- [ ] **Step 5: Commit vercel.json**

```bash
cd /home/evan/projects/spectral
git add site/vercel.json
git commit -m "feat(site): add Vercel deployment config"
git push origin feature/broker-enhancements
```

Vercel will auto-deploy on every push to the default branch once the project is connected.

---

## Self-Review

**Spec coverage check:**
- [x] Astro + Vercel → Task 1, 13
- [x] Logo (ghost with tile, overflow, wink) → Task 3, 6
- [x] Favicon → Task 3
- [x] Nav (sticky, logo, links, download) → Task 4
- [x] Hero (badge, headline, disappear animation, dual CTA) → Task 5
- [x] Features (9 cards, ghost hover reveal) → Task 6, 7
- [x] How it works (3 steps, connecting line) → Task 8
- [x] Privacy (checklist + level bars) → Task 9
- [x] Open Source dark section → Task 10
- [x] CTA banner → Task 11
- [x] Footer → Task 11
- [x] No emojis, Lucide icons throughout → enforced in all components
- [x] Copy accuracy (130+, PII wording, adtech wording) → baked into component data

**Type consistency check:**
- `Logo.astro` props: `width`, `height` — used consistently in Nav and FeatureCard (inlined)
- `FeatureCard.astro` props: `icon`, `title`, `description` — matches Features.astro card array
- All Lucide icon names match those registered in Layout.astro's `createIcons` call: `scan-search`, `mail-x`, `brain-circuit`, `lock-keyhole`, `filter-x`, `bell-ring`, `cookie`, `scan-text`, `target`, `shield-check`, `download`, `github`, `check-circle-2`, `code-2`, `git-pull-request`, `package`

**Placeholder check:** No TBDs, TODOs, or vague steps found. All code blocks are complete.
