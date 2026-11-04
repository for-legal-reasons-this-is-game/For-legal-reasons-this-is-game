# Sprint exercises: PlaceOrder

Small Rustlings-style exercises for PR A (PlaceOrder) in [../SPRINT.md](../SPRINT.md), already updated to the contract remake (#53, PR #56). Each file has `todo!()` bodies and tests. When a file passes, you know how to write that part of the real PR.

Local only, like the other docs here.

## Running

`proto` needs `protoc`, so start from the engine's dev shell:

```sh
cd tradingengine
nix develop
cd docs/exercises
cargo run            # like `rustlings`: watch mode
```

Watch mode opens the first unfinished exercise and runs it again every time you save the file. Once it passes, type `n` and Enter to go to the next one. The other keys: `h` hint, `l` list, `r` run again, `q` quit.

Other commands:

| Command | What it does |
|---|---|
| `cargo run -- list` | every exercise, done or todo |
| `cargo run -- hint` | the hint for the current exercise |
| `cargo run -- reset` | forget the progress (kept in `.sprint-progress`); your files stay as they are |
| `cargo test --test ex03_validate_place` | run one exercise once |
| `./check.sh` | run them in order without watching, stop at the first failure |
| `./check.sh --solutions` | check the reference answers |

The first run compiles the engine and every exercise, so it takes a while. After that, each save only rebuilds the file you changed.

Edit only `exercises/`. The answers are in `solutions/`. The hints are in [HINTS.md](HINTS.md). `src/` holds shared fixtures and the runner.

If you change a solution, run `python3 gen_exercises.py` to regenerate `exercises/`. Every body between `//>>` and `//<<` becomes `todo!()`. Note: this overwrites your work in `exercises/`.

## The exercises

| # | Exercise | Sprint step | Where it goes in the real code |
|---|---|---|---|
| 00 | `contract_quiz` | What #53 changed for PlaceOrder | nothing, it's the rules around `INTERNAL_ERROR` |
| 01 | `wire_enums` | PR A step 3 | `src/wire.rs`: `i32` to `Side`, `OrderType`, `TimeInForce` |
| 02 | `required_fields` | PR A step 3 | `src/wire.rs`: ids, `limit_price`, the `base_quantity` arm |
| 03 | `validate_place` | PR A steps 2 and 3 | the place parser, called by `Engine::place_order` |
| 04 | `filled_quote` | PR A step 5 | `filled_quote` is the sum of `quote_for` per fill |
| 05 | `reject_mapping` | PR A, errors | `EngineError` to `RejectReason`, panic otherwise |
| 06 | `place_response` | PR A step 5 | building `PlaceOrderResponse`, `Open` reported as `New` |
| 07 | `market_place` | PR A step 4 | `MarketInfo::place` |

The exercises don't decide any open question for you. Where a test would need an answer that isn't settled yet (`DuplicateOrderId`), the test checks only what is certain, and the file header says what to raise with the team.

## What is not here

- Multithreading (PR B: market tasks, Redis, shutdown with `INTERNAL_ERROR`). Someone else owns it.
- Cancel, modify, the command cache, and the event stream. Those are later slices.
