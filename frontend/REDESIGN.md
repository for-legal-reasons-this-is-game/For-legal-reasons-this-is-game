# Frontend redesign plan

Working notes for turning the test UI into the real frontend, over several prompts.
Branch: `ai/frontend-redesign` (made from `ai/frontend-endpoints`, which has a component for every
backend route). Update this file at the end of every prompt: tick the checklist, add to the log,
and move answered questions into "Decisions".

## Brief

From the user, 2026-11-22: "make it functional with a modern style, with as many pages as
necessary." Use **TradingView Lightweight Charts**, adopt **shadcn/ui**, and **test with
Playwright** when we finish. No PRs for now. The design follows Anthropic's `frontend-design`
skill (plan, review against the brief, build, critique with screenshots).

**Subject (proposed, confirm):** "For legal reasons this is game", a paper-trading exchange.
Spot markets such as BTC-USD run on a real matching engine and a double-entry ledger
(TigerBeetle), with play money.
**Audience (proposed, confirm):** students and evaluators trying the exchange, and the team as
admins (ledgers, markets).
**Primary job (proposed, confirm):** place an order in a market and follow it, and see your
balances. Secondary: admin setup.

## Stack decisions

| What | Choice | Notes |
|---|---|---|
| Styling | Tailwind CSS 4 (`tailwindcss`, `@tailwindcss/vite` 4.3.x) | Required by shadcn. Replaces `index.css`/`App.css`. |
| Components | shadcn/ui (CLI `shadcn` 4.23.x), icons `lucide-react` | Components are copied into `src/components/ui/` and are ours to edit. |
| Import alias | `@/` → `src/` | shadcn needs it. TypeScript 6 deprecates `baseUrl`, so use `paths` only (verify in phase 1). |
| Routing | React Router 8 (`react-router`, declarative mode) | Needs React ≥ 19.2.7 (we have ^19.2.8). nginx already falls back to `index.html`. |
| Charts | Lightweight Charts 5.3 (Apache-2.0) | Candlesticks + volume. Requires its attribution notice (see Licenses). |
| Data | TanStack Query + `src/api` (unchanged) | Add `queryOptions` factories so query keys live in one place. |
| Fonts | Archivo variable, self-hosted (`@fontsource-variable/archivo`, OFL) | Check that the width axis (`wdth.css`) is importable in phase 1. |
| Tests | Playwright (`@playwright/test` 1.65) | Phase 6. Downloading its browser (about 150 MB) needs the user's OK first. |

## Design direction: "Ledger paper"

### Why this direction

Paper trading on a double-entry ledger: the app already shows debits, credits and holds. The
look borrows from columnar ledger paper (cool, ruled, precise), not from crypto exchanges.
Numbers are the content, so typography and alignment do most of the work.

### Palette (light theme, default follows the system setting)

| Name | Hex | Role |
|---|---|---|
| Ledger Paper | `#F4F6F1` | app background (cool, faintly green; not the warm cream default) |
| Sheet | `#FFFFFF` | panels, tables, inputs |
| Ledger Ink | `#16231D` | text (a dark green-ink, not a near-black stand-in for black) |
| Ruling | `#D6DED4` | table rules and dividers that carry structure |
| Ink Blue | `#2348C8` | actions, links, focus ring, selection |
| Gain / Loss | `#11875A` / `#C42F3C` | price direction and buy/sell only |

Dark theme ("ledger at night"), derived from the same hues: Paper `#101814`, Sheet `#16201B`,
Ink `#E6EEE8`, Ruling `#2A3830`, Ink Blue `#7C9BFF`, Gain `#3CCB8A`, Loss `#F06A73`.
Check every pair for WCAG AA contrast in phase 1, and adjust the values (not the idea).

### Type

One family, **Archivo** (variable weight and width), used in three widths:
- **Expanded, heavy:** the market price on the trade page (the one bold element, see below) and
  page titles.
- **Normal:** body text, forms, buttons.
- **Semi-condensed:** dense tables (orders, trades, transfers).

Tabular figures (`font-variant-numeric: tabular-nums`) for every number, right-aligned in
columns. No monospace for data. Scale (Bringhurst): 12 / 14 / 16 / 18 / 21 / 24 / 36 / 48 px.
Sentence case everywhere, no all-caps labels.

### The one memorable element

The **price header of the trade page**: the last price in Archivo Expanded at 48 px with tabular
figures. When the price changes, only the digits that changed flash in Gain or Loss for about
600 ms (motion that answers new data; off with `prefers-reduced-motion`). Everything around it
stays quiet.

### Layout

App shell: top bar with the wordmark, a market switcher, the acting user and the theme toggle,
plus a navigation rail on the left (a sheet on mobile).

Trade page, desktop (≥ 1280 px):

```
+--------------------------------------------------------------------------------+
| For legal reasons this is game   [BTC-USD v]            Play money  [alice v] o |
+-------+------------------------------------------------------------------------+
| Mkts  | BTC-USD  65,450.00   +900.00 (+1.39%)   24h high 65,450  low 64,550     |
| Trade +--------------------------------------------------+---------------------+
| Port. |                                                  |  Buy  |  Sell       |
| Orders|          candlestick chart + volume              |  Limit, good till   |
| Users |          [1m 5m 15m 1h 1d]                       |  cancelled          |
| Admin |                                                  |  Price   65,000.00  |
|       +--------------------------------------------------+  Amount  0.2000 BTC |
|       | Recent trades | Market orders | My orders        |  Total   13,000 USD |
|       | time       price        amount       side        |  [ Place buy order ]|
|       | 14:02:11   65,450.00    0.0500       buy         |  Rules: min 0.0001  |
+-------+--------------------------------------------------+---------------------+
```

Mobile (360 px): price header, chart, then tabs; the order form opens from sticky Buy and Sell
buttons as a bottom sheet.

Markets page: one table (market, last price, 24 h change, high, low, volume, status). A row
opens its trade page. No hero, no cards: the table is the page.

Portfolio and account pages show balances as a ledger: debits and credits in two columns,
holds (pending) apart from posted amounts.

### Principles

1. Numbers first: tabular figures, right-aligned, each asset's own decimals, no floating-point
   maths on money (amounts stay decimal strings; format with `Intl.NumberFormat`, which accepts
   strings exactly).
2. Rules encode structure (rows, debit|credit), never decoration. Radius 6 px for controls,
   10 px for panels. Shadows only on overlays (dialogs, menus).
3. Colour has fixed meanings: Gain/Loss for direction, Ink Blue for actions and focus. Status
   badges stay muted.
4. Every list and form has loading (skeleton rows), empty (says what to do next) and error
   (says what happened and how to fix it) states.
5. Copy from the user's side, in plain verbs: "Place buy order", "Deposit", "Open account".
   An action keeps its name in buttons, toasts and errors. "Play money" stays visible (it's the
   project's own joke, and it's true).
6. Quality floor: WCAG AA contrast, visible keyboard focus, labelled inputs, reduced motion,
   works from 360 px wide.

### Review against the brief (frontend-design, second pass)

What changed from the first draft, and why:
- **Dark navy with an amber accent** was my first palette. It reads like any crypto exchange
  (close to Binance's black and yellow), so it was replaced by "ledger paper", which comes from
  this project's own subject: paper trading on a double-entry ledger.
- **Monospace prices** is a listed generic tell; replaced by Archivo's tabular figures.
- **Market tiles as cards** is the "SaaS card kit" default; markets are comparable rows of
  numbers, so a table.
- **Green as the brand colour** would collide with "price went up"; the accent is Ink Blue.
- **Ruled lines** risk the broadsheet default (hairlines everywhere, no radius); rules are only
  used where they carry structure, and controls keep a small radius.

## Pages and routes

| Route | Page | Endpoints |
|---|---|---|
| `/` | redirects to `/markets` | |
| `/markets` | Markets table | `GET /markets`, `GET /markets/{s}/ticker` per market |
| `/markets/:symbol` | Trade page | ticker, trades (for candles), market orders, `POST /users/{id}/orders`, user orders |
| `/portfolio` | Your accounts and balances | positions, `POST /users/{id}/accounts`, deposit |
| `/accounts/:id` | Account: balance, transfers, deposit, close/reopen | account, position, transfers, deposit, close, reopen |
| `/orders` | Your orders (market and status filters, paging) | `GET /users/{id}/orders` |
| `/orders/:id` | Order and its fills | `GET /orders/{id}`, `GET /orders/{id}/trades` |
| `/users` | Users: create, switch the acting user | `GET/POST /users`, `GET /users/{id}` |
| `/admin/ledgers` | Ledgers: create, enable/disable | ledgers endpoints |
| `/admin/markets` | Markets: create, list with status | `GET/POST /markets` |
| `*` | Not found | |

There's no login yet (Keycloak is planned), so the top bar has an **acting user** switcher,
kept in a React context and remembered in `localStorage`. It's replaced by login later.

## Code structure (target)

```
src/
  app/          router, providers (Query, theme, acting user), layout (shell, nav)
  pages/        one file per route
  features/     per domain: market/, order/, account/, user/, admin/ (components + hooks)
  components/ui shadcn components
  lib/          cn(), format.ts (decimal strings), candles.ts (trades -> candles)
  api/          unchanged: client.ts, index.ts, plus queries.ts (queryOptions factories)
  types/        unchanged: copies of backend/bindings
```

The current test UI stays reachable at `/dev` until each part has a replacement, then it's
deleted (phase 5).

### Chart data

There's no candles endpoint, so candles are built in the browser from
`GET /markets/{symbol}/trades` (newest first, at most 100 per page): fetch the latest few pages,
group by interval into OHLC + volume, refetch every 5 s. A backend candles endpoint (or the
engine's market-data stream) would replace this later.

## Backend dependencies and limits (Nov 2026)

These shape the UI. Ask Alvaro/Mehras where needed:
- **No `GET /users/{id}/accounts`.** The portfolio can't list a user's accounts. For now: the
  accounts created in this browser, plus those found in the user's orders, plus "add by ID".
- **The engine only accepts GTC limit orders** (`tradingengine/src/place.rs`). Keep the other
  order types in the form behind one flag (`ENGINE_SUPPORTS`), shown as "not available yet",
  easy to switch on.
- **The consumer can't run** until the engine implements `SubscribeEvents`. On the real stack
  there are no trades, so the chart and ticker are empty, and stored orders have no status or
  fills. Development and tests use seeded trades.
- **No cancel or modify route** in the backend yet (the engine has cancel). Leave room for a
  "Cancel order" action, but don't build it until the route exists.
- **Errors are bare status codes.** A 400 can't say "unsupported order type". Messages stay
  per-status (see `errorMessage` overrides).

## Phases

Each phase should fit in one prompt and end with: type-check, lint and build passing, a
screenshot review of what changed, and this file updated (plus a local commit, if question 6 is
answered yes).

### Phase 0: plan and notes
- [x] Branch `ai/frontend-redesign` from `ai/frontend-endpoints`
- [x] This plan, design direction and review
- [x] Memory note pointing here
- [ ] User confirms: subject/audience/job, the "ledger paper" direction, open questions below

### Phase 1: foundation
- [ ] Tailwind 4 + `@/` alias + `shadcn init`; first components (button, input, select, label,
      table, tabs, dialog, sheet, dropdown-menu, badge, skeleton, sonner, tooltip)
- [ ] Theme: palette as shadcn CSS variables (light + dark), toggle, system default; AA check
- [ ] Archivo self-hosted, type scale, tabular figures utility
- [ ] React Router: shell (top bar, nav rail, mobile sheet), stub page for every route, 404
- [ ] Acting-user context + switcher; old UI moved to `/dev`
- [ ] Screenshots: shell in light, dark, and mobile

### Phase 2: markets and trading
- [ ] `api/queries.ts` (queryOptions), `lib/format.ts`
- [ ] Markets table with tickers
- [ ] Trade page: price header (the memorable element), chart with intervals and polling
- [ ] Order form on shadcn (port `CreateOrder`, keep its key handling), `ENGINE_SUPPORTS` flag
- [ ] Tabs: recent trades, market orders, my orders in this market

### Phase 3: portfolio and accounts
- [ ] Portfolio: accounts with balances (interim account list), open account, deposit dialogs
- [ ] Account page: ledger view, transfers with paging, close/reopen with confirmation

### Phase 4: orders, users, admin
- [ ] Orders page (filters, paging) and order page with fills
- [ ] Users page (create, act as) and user lookup
- [ ] Admin: ledgers (create, enable) and markets (create, list)

### Phase 5: polish and critique
- [ ] Loading, empty and error states everywhere; toasts for every action
- [ ] Responsive pass at 360 / 768 / 1280 px; keyboard and screen-reader pass; reduced motion
- [ ] Delete `/dev` and the old components
- [ ] Screenshot critique; "remove one accessory"

### Phase 6: Playwright
- [ ] `@playwright/test` + Chromium (ask before downloading)
- [ ] Script for a throwaway test stack: Postgres, TigerBeetle, Redis in containers; backend and
      engine run natively; seeded trades
- [ ] Tests: every route renders, trade flow (place a real order), error messages, deposits,
      admin forms, mobile layout, theme toggle
- [ ] Run, fix, document how to run them

## Open questions (defaults apply if not answered)

1. Subject, audience and primary job above: right? (default: yes)
2. "Ledger paper" direction, light by default with a dark theme: OK? (default: yes)
3. Name in the UI: "For legal reasons this is game" in full? (default: yes, shortened on mobile)
4. Interim account list until the backend has `GET /users/{id}/accounts`: OK? (default: yes)
5. Show unsupported order types as "not available yet" rather than hiding them? (default: show)
6. Commit locally at the end of each phase (never push unless asked)? (no default: needs a yes)

## Decisions

(Move answered questions here, with the date.)

## Licenses

- Lightweight Charts (Apache-2.0). Its README says the license requires two things on a page
  users can see: the attribution notice from its NOTICE file ("TradingView Lightweight Charts™,
  Copyright (c) 2025 TradingView, Inc.") and a link to https://www.tradingview.com/. The
  `attributionLogo` chart option (on by default) covers the link; the notice goes in the app
  footer or an About page.
- Archivo: SIL Open Font License 1.1.

## Log

- 2026-11-22: plan written (phase 0). The `frontend-design` plugin is enabled on the account;
  until it shows up in a session, its SKILL.md (anthropics/claude-plugins-official,
  `plugins/frontend-design`) was followed directly.
