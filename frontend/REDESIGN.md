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
| Gain / Loss | `#0D7C51` / `#C42F3C` | price direction and buy/sell only |
| Input border | `#76867C` | outline of inputs (needs 3:1 against white and paper) |

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
|       | Recent trades | All orders | My orders           |  Total   13,000 USD |
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
                (phase 2: market/ = table, price header, chart, activity tabs; order/ = ticket,
                orders table, labels; account/ = known accounts, add-by-ID dialog)
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
- [x] User confirms: subject/audience/job, the "ledger paper" direction, open questions below

### Phase 1: foundation (done 2026-11-22)
- [x] Tailwind 4 + `@/` alias + `shadcn init`; first components (button, input, select, label,
      table, tabs, dialog, sheet, dropdown-menu, badge, skeleton, sonner, tooltip, sidebar,
      separator)
- [x] Theme: palette as shadcn CSS variables (light + dark), toggle, system default; AA check
- [x] Archivo self-hosted, type scale, tabular figures utility
- [x] React Router: shell (top bar, nav rail, mobile sheet), stub page for every route, 404
- [x] Acting-user context + switcher; old UI moved to `/dev`
- [x] Screenshots: shell in light, dark, and mobile

### Phase 2: markets and trading (done 2026-11-23)
- [x] `api/queries.ts` (queryOptions), `lib/format.ts`, `lib/decimal.ts`, `lib/candles.ts`
- [x] Markets table with tickers
- [x] Trade page: price header (the memorable element), chart with intervals and polling
- [x] Order form on shadcn (port `CreateOrder`, keep its key handling), `ENGINE_SUPPORTS` flag
- [x] Tabs: recent trades, all orders, my orders in this market
- [x] Known accounts (decision 4): remembered per user, found in orders, add by ID

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
- [ ] Delete `/dev`, the old components, `src/dev/legacy.css` and `pages/PagePlaceholder.tsx`
- [ ] Split the bundle per route (`React.lazy`): it's 513 kB after phase 1, over Vite's
      500 kB warning, and the chart library will add more
- [ ] Toasts: the action button ("View order") is ink, make it Ink Blue; on phones the toast
      covers the sticky Buy/Sell bar for its 4 seconds (sonner's `mobileOffset`)
- [ ] Screenshot critique; "remove one accessory"

### Phase 6: Playwright
- [ ] `@playwright/test` + Chromium (ask before downloading)
- [ ] Script for a throwaway test stack: Postgres, TigerBeetle, Redis in containers; backend and
      engine run natively; seeded trades
- [ ] Tests: every route renders, trade flow (place a real order), error messages, deposits,
      admin forms, mobile layout, theme toggle
- [ ] Run, fix, document how to run them

## Open questions (defaults apply if not answered)

(None open.)

## Decisions

All answered by the user on 2026-11-22 ("yes to all defaults including 6"):
1. Subject, audience and primary job as written in the brief.
2. "Ledger paper" direction; the theme follows the system setting, with a toggle.
3. The UI shows the full name "For legal reasons this is game" (shortened on mobile).
4. Until the backend has `GET /users/{id}/accounts`, the portfolio lists the accounts created
   in this browser, those found in the user's orders, and accounts added by ID.
5. Unsupported order types are shown as "not available yet", not hidden.
6. Commit locally at the end of every phase. Never push unless asked.

## Notes for later phases

Things decided or learned while building, which the next phases rely on:
- **shadcn setup:** preset Nova (style `radix-nova`, Lucide icons), its Geist font replaced by
  Archivo. Add components with `npx shadcn@4.23.0 add <name> -y`. The files in
  `src/components/ui/` keep shadcn's own formatting (2 spaces, no semicolons) so updates from
  the CLI stay easy to compare; our files use the project's Prettier settings (4 spaces).
- **`cn` package:** shadcn now ships `cn` (npm `cn`, by shadcn, MIT) instead of
  `clsx` + `tailwind-merge`. `src/lib/utils.ts` re-exports it.
- **Theme:** `next-themes` (came with the toast component) stores "light", "dark" or "system"
  under the `theme` key; `index.html` applies the class early so dark visits don't flash.
- **Radius:** `--radius` is 6 px. Controls use `rounded-lg` (6 px), panels `rounded-2xl`.
- **Colours** were adjusted for WCAG AA in phase 1 (Gain `#0D7C51`, input border `#76867C`).
  The checker script lives only in the session scratchpad; the rule is in `index.css`.
- **Type scale:** Tailwind's sizes, except `text-xl` is 21 px. Skip `text-3xl` (30 px is off
  the scale). Widths: `font-stretch-expanded` (titles, the price), `font-stretch-semi-condensed`
  (dense tables). Tabular figures are on for the whole page.
- **Changed shadcn files:** `hooks/use-mobile.ts` uses `useSyncExternalStore` (the original
  set state inside an effect, which our lint rules reject). ESLint's fast-refresh rule is off
  for `src/components/ui/` only.
- **Acting user:** `useActingUser()` from `@/app/acting-user/context`; remembered in
  `localStorage` under `acting-user-id`.
- **Sidebar header:** the mark sits in a 32 px box and the name wraps with
  `whitespace-normal!`, so both the icon rail and the full name look right.
- **Query keys** live in `api/queries.ts` (`marketQueries`, `userQueries`,
  `accountQueries`). Market data polls every 5 s (`LIVE_MS`). The old components share the
  `["account", id]` and `["position", id]` keys, so invalidations reach both UIs.
- **Money maths:** `lib/decimal.ts` works in whole units with BigInt, like the backend:
  `toUnits`/`fromUnits`, `quoteFor` (rounds down, null where the backend can't compute it),
  `feeCap` (rounds up). The ticket's "Held for this order" is the backend's
  `reservation_amount()`: a buy holds value + fee at the higher of the two rates, a sell holds
  the base amount. Never format money through `Number`.
- **Formatting:** `lib/format.ts`: `formatDecimal` (the asset's own decimals),
  `formatTrimmed` (rules and hints), `formatWhen` (time today, date and time otherwise).
- **Available balance** = `net_posted - debits_pending`: an order's hold is a posted transfer
  to its hold account, so the posted balance is already what's free.
- **Known accounts** (`features/account/`): `rememberAccount(userID, id)` stores IDs in
  localStorage per user (`known-accounts:<user id>`); `useUserAccounts(userID)` adds the
  accounts named in the user's newest 100 orders and fetches each one. Phase 3 should call
  `rememberAccount` when it opens an account.
- **Order state:** `OrderStateBadge` combines `submission` and `status`. While the consumer
  can't run, accepted orders have no status and show as "Accepted".
- **Order form:** `features/order/OrderTicket.tsx`. Validation mirrors the backend's 400 checks
  (decimals, minimums, steps, min value) and shows after the first submit. The idempotency key
  rules are CreateOrder's. Over-balance is a warning, not a block (balances can be stale).
  Its summary ends like a ledger page: value, fee, a double rule, then the held total. Step
  hints only appear when a market's step is coarser than its decimals.
- **Chart:** candles are built in the browser (`lib/candles.ts`) from the newest 500 trades,
  with times shifted to the viewer's time zone (Lightweight Charts shows UTC otherwise). Colours
  come from the CSS variables and follow the theme through a MutationObserver on `<html>`. The
  chart's container is `isolate` so the library's z-indexes stay under our overlays.
- **Trade page layout** uses a container query (`@container`, `@4xl:`): the ticket sits
  beside the chart when the page area is at least 896 px wide (so collapsing the sidebar can
  bring it back), otherwise it opens from a sticky Buy/Sell bar as a bottom sheet. The side
  ticket is sticky and scrolls inside itself on short screens.
- **Portals and forms:** a dialog inside a form still sends its events up the React tree
  (portals bubble through React parents). `AddAccountDialog` stops its submit from reaching
  the order form; do the same for any dialog placed inside a form.
- **Buttons on Gain/Loss:** `text-gain-foreground` / `text-loss-foreground` (white in light
  mode, dark ink in dark mode: white on the dark theme's Gain is only 2.1:1).
- **Bundle:** 758 kB after phase 2 (the chart library is most of the growth); phase 5 splits it.
- **npm audit** reports 13 findings, all in build tools (ESLint, typescript-eslint, the shadcn
  CLI and its `zod`), none in the code the browser runs. Don't run `npm audit fix` blindly:
  it changes dependency versions.

## Licenses

- Lightweight Charts (Apache-2.0). Its README says the license requires two things on a page
  users can see: the attribution notice from its NOTICE file ("TradingView Lightweight Charts™,
  Copyright (c) 2025 TradingView, Inc.") and a link to https://www.tradingview.com/. The
  `attributionLogo` chart option (on by default) covers the link; the notice is printed under
  the chart (`features/market/PriceChart.tsx`), with a text link to TradingView too.
- Archivo: SIL Open Font License 1.1.

## Log

- 2026-11-22: plan written (phase 0). The `frontend-design` plugin is enabled on the account;
  until it shows up in a session, its SKILL.md (anthropics/claude-plugins-official,
  `plugins/frontend-design`) was followed directly.
- 2026-11-22: phase 1 done. Tailwind 4, shadcn (Nova), Archivo, the "ledger paper" theme in
  light and dark (all pairs AA), React Router with the app shell and a page for every route,
  the acting-user switcher, the old UI at `/dev`. Checked in the browser at 1280 px (light,
  dark, icon rail), tablet width and 375 px (sheet). Fixed on the way: the top bar's divider
  sat at the top, the icon rail showed a clipped letter, and the full name was truncated.
  Next: phase 2 (markets and the trade page).
- 2026-11-23: phase 2 done. Markets table with live tickers; trade page with the flashing
  price header, Lightweight Charts candles and volume (5 intervals, local time, theme colours),
  tabs for recent trades, all orders and my orders (filters, paging), and the order ticket
  (exact totals, validation, "not available yet" order types, known accounts and add by ID,
  toasts). Tested on a throwaway stack (Postgres, TigerBeetle, Redis in containers, backend and
  engine native, 509 seeded trades): placed a real buy and a real sell (201, held amounts and
  balances matched the backend to the cent), a 409 for too little money, the flash on a new
  trade, light and dark, 1280 / 1024 / 375 px, halted and empty markets, no user, unknown
  market. Fixed on the way: the dialog's Enter also submitted the order form (portal events
  bubble), the empty-chart message hidden under the canvas, a stray volume label on the price
  axis, UTC times on the chart, volume colours not following a theme switch, and the time in
  force select overflowing the ticket. Next: phase 3 (portfolio and accounts).
