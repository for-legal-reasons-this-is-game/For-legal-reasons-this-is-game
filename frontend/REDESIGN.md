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
| `/orders` | Your orders (market and status filters in the address, paging) | `GET /users/{id}/orders` |
| `/orders/:id` | Order: details, fills with totals | `GET /orders/{id}`, `GET /orders/{id}/trades` |
| `/users` | Users: create, search, switch the acting user | `GET/POST /users` |
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

The first test UI (one form per backend route) stayed at `/dev` until every part had a
replacement, and was deleted in phase 5.

### Chart data

There's no candles endpoint, so candles are built in the browser from
`GET /markets/{symbol}/trades` (newest first, at most 100 per page): fetch the latest few pages,
group by interval into OHLC + volume, refetch every 5 s. A backend candles endpoint (or the
engine's market-data stream) would replace this later.

## Backend dependencies and limits (Nov 2026)

**Scope until the evaluation** (from the user, 2026-11-23, corrected 2026-11-25): of the
engine's seven RPCs (`proto/service.proto`), the backend implements `SetMarket`, `PlaceOrder`,
`CancelOrder`, `SubscribeEvents` and `SubscribeMarketData` before the evaluation. `ModifyOrder`,
`CancelOrders` and order types other than GTC limit come after it. For the UI that means:
- Trades, fills, order statuses and settlement work at the evaluation (the consumer runs on
  `SubscribeEvents`), so the trade page fills up as people trade. Until then, development
  stacks use seeded trades.
- The reply to `POST /users/{id}/orders` carries the engine's immediate status and fills; the
  toast shows them ("Buy order filled"). Stored orders get theirs from the consumer.
- `SubscribeMarketData` is an order book feed (`BookSnapshot`, `BookDelta` in
  `proto/events.proto`). An order book panel can come once the backend exposes it (open
  question 3).
- Cancelling works once the backend has a cancel route. On 2026-11-25 there's none yet
  (`backend/src/service.rs` has the client call). `POST /markets` does call `SetMarket`
  (`advance_market`): 201 with "trading" when the engine confirms in time, otherwise 202 with
  "halted" while the relay keeps asking (checked on 2026-11-25; an earlier note here said it
  didn't).
- **Closing an account with open orders calls `CancelOrders`** (`close_account` in
  `backend/src/v1.rs`), which comes after the evaluation: until then, such a close fails
  (502). Accounts without open orders close fine.
- How the UI shows the rest: decision 7.

These shape the UI. Ask Alvaro/Mehras where needed:
- **No `GET /users/{id}/accounts`.** The portfolio can't list a user's accounts. For now: the
  accounts created in this browser, plus those found in the user's orders, plus "add by ID".
- **The engine only accepts GTC limit orders** (`tradingengine/src/place.rs`). Keep the other
  order types in the form behind one flag (`ENGINE_SUPPORTS`), shown as "not available yet",
  easy to switch on.
- **The consumer can't run** until the engine implements `SubscribeEvents` (planned before the
  evaluation). Until then, there are no trades on a real stack, so the chart and ticker are
  empty and stored orders have no status or fills. Development and tests use seeded trades.
- **The API doesn't say whether an account is closed.** Closing sets a flag in TigerBeetle
  only; accounts and positions report "active" or "processing". The account page offers both
  close and reopen (both are safe to repeat), and the badge can't show "Closed" (open
  question 4).
- **No cancel route** in the backend yet. Cancel is in scope for the evaluation, so the UI gets
  a "Cancel order" action (follow-up below), wired when the route exists. Modify comes after
  the evaluation.
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

### Phase 2 follow-up: evaluation scope (done 2026-11-25)
- [x] `lib/engine-support.ts` lists what isn't available yet in one place (order types, and a
      pointer to the cancel button)
- [x] ~~Trade history "not available yet"~~: dropped, `SubscribeEvents` is in scope for the
      evaluation, so "No trades yet ... appear when orders meet" is true there
- [x] Order lists: "Accepted", "Placing" and "Rejected" explain themselves in a tooltip
      (rejections say why). The toast after placing shows the engine's immediate answer
      ("Buy order filled: 0.05 BTC for 3,200.00 USD", "partly filled", "cancelled" + reason)
- [x] "Cancel" on your own open orders (My orders): disabled with "Cancelling orders isn't
      available yet" until the route exists (`features/order/CancelOrderButton.tsx`). The order
      page gets it in phase 4.

### Phase 3: portfolio and accounts (done 2026-11-25)
- [x] Portfolio: accounts with what each can spend (interim account list), totals per asset,
      open account and deposit dialogs, add by ID
- [x] Account page: ledger view (out | in, posted and pending, double rule, balance), transfers
      as ledger lines with paging, close/reopen with confirmation, copy ID
- [x] Order form: "Open one" when an account is missing, "Deposit" under the available
      balance, so a new user can go from nothing to an order on the trade page

### Phase 4: orders, users, admin (done 2026-11-25)
- [x] Orders page: every market, market and status filters kept in the address
      (`?market=BTC-USD&status=open`), paging; filtered to one market it looks like the trade
      page's list. Order page: a title in words ("Buy 0.1 BTC at 63,500.00 USD"), why it
      ended (rejected, cancelled, expired), details ruled like a ledger page (filled amount
      and value, average price, what it held, fee rates, both accounts), fills with maker or
      taker, fee and totals under a double rule, the disabled "Cancel order". Fills aren't
      "not available yet" any more: `SubscribeEvents` is in scope, so they show for real
- [x] Users page: everyone by name, search by name or ID (it replaces the old ID lookup:
      `GET /users` already returns everyone), "Act as", create user (with nobody chosen yet,
      the app acts as the new user; otherwise the toast offers to switch)
- [x] Admin: ledgers (create, a switch per ledger to turn it on or off) and market setup
      (create with checks that mirror the backend's, list with order rules and fees; a market
      waiting for the engine says so and turns to "Trading" by itself)

### Phase 5: polish and critique (done 2026-11-25)
- [x] Loading, empty and error states everywhere; toasts for every action. Added: an error
      screen for a page that crashes or fails to download (`components/ErrorBoundary`), a
      placeholder while a page's code loads, the trade page's price says when it couldn't
      load (it said "No trades yet"), an empty ledgers list
- [x] Responsive pass at 375 / 768 / 1280 px; keyboard and screen-reader pass; reduced motion
      (already global since phase 1). Added: a "Skip to content" link, a navigation landmark
      for the sidebar, a solid focus ring on links (the theme's half-transparent one was under
      3:1); every control on every page has a name, one h1 per page
- [x] Deleted `/dev`, the old components, `src/dev/legacy.css`, the unused template images
      (`src/assets`) and `pages/PagePlaceholder.tsx` (phase 4). Their explanations for
      newcomers (`as`, generics, `useState(() => ...)`) moved into the files that use them
- [x] Split the bundle per page (`React.lazy`): the app's main file is 372 kB (825 kB
      before), the trade page with the chart library 202 kB, the other pages 3 to 13 kB.
      Vite's 500 kB warning is gone
- [x] Toasts: the action button is Ink Blue; on the trade page they rise above the Buy/Sell
      bar while it shows (it measures itself, see "Toasts" below)
- [x] Phones: the order lists become two-line rows ("Buy 0.25 BTC at 66,000.00 USD" over the
      time, status and Cancel on the right) when their box is narrow; the fills, recent
      trades, transfers, markets and market setup tables drop or move columns by their own
      width. Nothing scrolls sideways at 375 px any more
- [x] "Users" stays under Admin; instead, every "choose who you're acting as" message links
      to creating a user, and the top bar's menu says "Create a user" while there are none
- [x] Screenshot critique; removed the green "Trading" badge beside the price, the trade
      page's one bold element: a trading market is the normal case, so only "Halted" or
      "Cancel only" show there now (the markets list on phones does the same). Also added the
      favicon `index.html` pointed to but never had (`public/favicon.svg`)

### Phase 6: Playwright
- [ ] `@playwright/test` + Chromium (ask before downloading)
- [ ] Script for a throwaway test stack: Postgres, TigerBeetle, Redis in containers; backend and
      engine run natively; seeded trades
- [ ] Tests: every route renders, trade flow (place a real order), error messages, deposits,
      admin forms, mobile layout, theme toggle
- [ ] Run, fix, document how to run them

## Open questions (defaults apply if not answered)

1. **Cancel route** (Alvaro): method, path and answer of the backend's cancel endpoint.
   Default: the button shows "not available yet" until the route is merged.
2. ~~Trade history during the evaluation~~: answered 2026-11-25, `SubscribeEvents` and
   `SubscribeMarketData` are in scope.
3. **Order book** (Alvaro): will the backend expose the engine's market data (a REST snapshot,
   or a WebSocket/SSE stream of `BookDelta`)? Default: no order book panel until it does.
4. **Closed accounts** (Alvaro): could `GET /accounts/{id}` or the position say whether an
   account is closed (e.g. `account_status: "closed"`)? Default: the account page offers both
   close and reopen, and says nothing about the state.
5. **The system user** (Alvaro): `GET /users` also returns the backend's own "system" user
   (it owns the fee and source accounts), and the API doesn't say which user that is
   (`is_system` isn't in `User`). Could `list_users` leave it out? Default: it's listed like
   everyone, in the users page and the top bar's menu.

## Decisions

All answered by the user on 2026-11-22 ("yes to all defaults including 6"):
1. Subject, audience and primary job as written in the brief.
2. "Ledger paper" direction; the theme follows the system setting, with a toggle.
3. The UI shows the full name "For legal reasons this is game" (shortened on mobile).
4. Until the backend has `GET /users/{id}/accounts`, the portfolio lists the accounts created
   in this browser, those found in the user's orders, and accounts added by ID.
5. Unsupported order types are shown as "not available yet", not hidden.
6. Commit locally at the end of every phase. Never push unless asked.

Chosen by Claude on 2026-11-23, after the user left the choice open:

7. Anything that needs what comes after the evaluation is shown as "not available yet"
   (decision 5's wording), never with made-up data. Each such feature sits behind a flag in
   `lib/engine-support.ts`, so switching it on later is a one-line change, and the real
   requests stay wired wherever the endpoint already exists.

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
- **Worktree:** since 2026-11-25 this branch lives in its own folder,
  `~/Desktop/github/transcendence-frontend` (`git worktree`), so the main checkout can stay
  on the user's branch. Work, build and commit there. The browser preview runs Vite from the
  worktree (`./node_modules/.bin/vite`) and opens its URL, instead of a launch config in the
  main checkout.
- **Accounts (phase 3):** `features/account/`. `OpenAccountDialog` remembers the new account
  (`rememberAccount`) and seeds the cache; `accountQueries.detail` polls every 2 s while an
  account is "processing". `DepositDialog` keeps CreateOrder's idempotency key rules.
  `availableUnits(position)` in `balance.ts` is the one place "what can be spent" is worked
  out. Transfers have TigerBeetle timestamps (nanoseconds): `formatLedgerWhen`.
- **Dialogs:** a dialog that starts on a chosen value should focus the next field itself
  (`onOpenAutoFocus`, see `OpenAccountDialog`); otherwise focus lands on the first control and
  Enter reopens a select. Dialogs inside forms or clickable rows stop their events
  (`stopPropagation`), see "Portals and forms".
- **Disabled buttons with a reason:** a disabled button gets no hover or focus, so its tooltip
  hangs on a focusable wrapper with `role="button"` and `aria-disabled` (`CancelOrderButton`).
- **Phones:** wide tables drop secondary columns and move the essentials into the first cell;
  a footer row uses one cell per column, not `colSpan`. Since phase 5 they do it by their
  own width, not the screen's (Tailwind container queries: `@container` on the box, then
  `@md:`, `@xl:`...), because the same table sits in a narrow column on the trade page and
  across a whole page elsewhere. `AccountsTable` still uses screen widths (`sm`/`md`); it's
  only ever page-wide. The order lists switch to two-line rows (`OrdersTable`'s `LAYOUTS`).
- **Bundle:** each page is its own file (`React.lazy` in `app/App.tsx`; the markets page,
  where the app opens, and the not-found page load with the app). 372 kB for the app after
  phase 5, 202 kB for the trade page (the chart library); 825 kB in one file after phase 4.
- **Errors:** `components/ErrorBoundary` (a class component: React has no hook for it) sits
  around the page in `AppShell`, keyed by the address so opening another page clears it.
  Failed requests are still shown by each page; this catches crashes and a page's code
  failing to download (after a deploy, for example).
- **Toasts:** the Buy/Sell bar on the trade page writes its height into `--toast-lift` on
  `<html>` (a ResizeObserver; 0 while it's hidden beside a wide page), and the Toaster's
  `offset`/`mobileOffset` add it to sonner's own distances.
- **Keyboard:** the first Tab stop is "Skip to content" (`AppShell`), which focuses the page
  without changing the address. Links get a solid Ink Blue focus outline (`index.css`).
- **No user chosen:** `features/user/ChooseUserHint` is the sentence every page uses, with a
  link to creating a user.
- **Shared pieces (phase 4):** `components/CopyButton` (copy an ID, with a toast),
  `components/EmptyState` (the dashed "nothing here yet, do this" box),
  `components/DecimalField` (a number input with its unit inside, moved out of the order
  form; `inputMode="numeric"` for whole numbers). `features/order/describe.ts` words an order
  the same way in the toast and the order page's title.
- **Orders:** `OrdersTable` takes `market` (one market: units in the headings) or `markets`
  (any market: a Market column and units in the cells). `orderQueries.detail` and the fills
  poll only while `isOpen(order)`. The fills table shows the latest 20 with "Show all" (the
  endpoint isn't paged); its totals always cover every fill. `rejectionText(order)` in
  `labels.ts` turns the backend's two own rejection details ("insufficient funds", "the
  account is closed...") into sentences: if `advance_order` changes those strings, the
  generic sentence shows instead.
- **Filters in the address:** the orders page keeps market and status in the query string
  (`useSearchParams`, with `replace` so Back still leaves the page). With a market in the
  address, it waits for the markets list before asking for orders, so an unknown market says
  "There's no market called ..." instead of the backend's ambiguous 404.
- **Rules only the frontend has:** user names are unique (the user menu shows only names),
  ledger symbols are capitals and digits (up to 12), market symbols are capitals, digits and
  dashes. The backend accepts any text for all three.
- **Ledger switch:** shows its new position while the request runs (TanStack's mutation
  `variables`), then updates the cached list and refetches the markets (they carry their
  ledgers).
- **Page grids:** a grid that turns into two columns on wide screens needs `grid-cols-1`
  below that, or a wide table inside widens the whole page on phones (its implicit column
  grows to the table's width).
- **Admin market list** polls every 5 s while a market is "halted" (waiting for the engine),
  with its own `refetchInterval` on the shared `["markets"]` key.
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
- Lucide icons (ISC). The favicon (`public/favicon.svg`) is Lucide's "notebook-text" icon,
  with its notice in a comment.

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
- 2026-11-23: the user set the scope until the evaluation (only `SetMarket`, `PlaceOrder`,
  `CancelOrder`) and left the presentation of the rest to me: decision 7, a phase 2 follow-up
  list, two open questions. Phase 3 is on hold until the user says go; no app code changed.
- 2026-11-25: the branch moved to its own worktree. The user corrected the scope:
  `SubscribeEvents` and `SubscribeMarketData` come before the evaluation, so trade history
  isn't marked unavailable. Phase 2 follow-ups done (explained order states, the engine's
  immediate answer in the toast, a disabled Cancel), and phase 3 done: portfolio, account
  page, open account, deposit, close/reopen, plus "Open one" and "Deposit" in the order form.
  Tested on a throwaway stack (backend and engine built from the worktree): opened an EUR
  account, deposited, closed it (a deposit then got the 409 message), reopened, deposited
  again; filled a buy against a resting sell (toast "Buy order filled"); opened and funded an
  ETH account from the order form; light, dark, 1280 / 1024 / 375 px; empty portfolio; unknown
  account. Fixed on the way: the open-account dialog focused the asset instead of the name, the
  My orders table overflowed its card, the portfolio table hid "Available" on phones, and the
  balance label squeezed the total. Next: phase 4 (orders, users, admin).
- 2026-11-25: phase 4 done: orders page, order page with fills, users page, ledgers and market
  setup. Tested on a throwaway stack (fills, a cancel reason and statuses set in Postgres,
  since the consumer can't run there): filters and the address, an unknown market, an order
  with three fills (totals and the average price checked by hand), one with 509 fills, a
  cancelled, a rejected and another user's order, an unknown order ID; created users (a
  duplicate name refused, "Act as" from the toast and the table), a ledger (a duplicate
  refused) and switched EUR off and on; created SOL-USD (confirmed at once) and, with the
  engine stopped, EUR-USD, which showed "Waiting for the trading engine" and turned to
  "Trading" by itself after the engine came back; a real order from the trade page (toast
  "0.002 BTC at 60,000.00 USD"); light, dark, 1280 and 375 px. Fixed on the way: the order
  title showed 8 decimals, "No fills yet" and "Nothing yet" on finished orders, "Held when
  placed" on rejected orders, the generic rejection text where the backend said why, a long
  fills list, the order page wider than a phone, a server error that outlived the field it
  was about, and an unknown market in the address briefly asking for its orders. The
  plan's note that `POST /markets` doesn't call `SetMarket` was wrong and is corrected. Next:
  phase 5 (polish), when the user says go.
- 2026-11-25: phase 5 done. Deleted the old test UI; each page loads lazily (main file 825 kB
  to 372 kB); an error screen, a loading placeholder, a skip link, a navigation landmark and
  solid focus rings on links; order lists as two-line rows and every wide table fitting by
  its own width (container queries); toasts above the Buy/Sell bar with an Ink Blue action;
  links to creating a user wherever nobody is chosen; the favicon. Tested on a throwaway
  stack at 375, 768 and 1280 px, light and dark: no page scrolls sideways or has a table
  wider than its box, the skip link and focus order, no unnamed control on any page, a
  halted market, a placed order's toast on a phone, and the error screen by stopping the
  dev server before opening a page. Docker Desktop hung on start again (VM "no route to
  host") and needed a quit and reopen. Critique: removed the "Trading" badge beside the
  price. Next: phase 6 (Playwright), when the user says go; it needs their OK to download
  Chromium.
