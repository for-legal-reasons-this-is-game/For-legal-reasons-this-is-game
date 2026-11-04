# Hints

One section per exercise. Read only the hint for the exercise you are stuck on. The reference answers are in `solutions/`.

## ex00_contract_quiz

Read the comment on `REJECT_REASON_INTERNAL_ERROR` in `proto/common.proto`: "while this command was still queued, so it was NOT processed". For the cache, look at step 2 of the signal handler under "Crash and shutdown" in `proto/how_to_use.md`.

## ex01_wire_enums

`proto::Side::try_from(raw)` returns `Result<proto::Side, prost::UnknownEnumValue>`. Match on both layers at once: `Ok(proto::Side::Buy) => ...`, `Ok(proto::Side::Unspecified) | Err(_) => ...`. Don't use the generated getter `request.side()`: it maps unknown numbers to `Unspecified` silently, which hides the difference you want to see in a test.

## ex02_required_fields

`required(field)` gives `Result<T, EngineError>`. Use `.map_err(|_| RejectReason::...)` to pick the reason. The price has two different failures, so convert in two steps: first `required`, then `Price::try_from`. For the amount, match on `Some(Amount::BaseQuantity(wire))`, `Some(Amount::QuoteQuantity(_))` and `None`.

## ex03_validate_place

Write the table from the file header as one block per row, in that order, each returning early. Work out three booleans once at the start: `is_market`, `is_stop` and `needs_limit`. Several rows compare whether a field is present with one of them: `request.limit_price.is_some() != needs_limit`. Use `let ... else` for the last row: `let (OrderType::Limit, TimeInForce::Gtc, Some(price)) = ... else { return Err(Unsupported) };`. The overflow row is `market.scale().quote_for(price, base)`.

## ex04_filled_quote

`Iterator::try_fold` with `QuoteQuantity::ZERO` as the start, and `?` on both `quote_for` and `checked_add`.

## ex05_reject_mapping

One `match error { ... }` with no `_`. Group variants with `|`. The panic arm can print the error: `panic!("{error}: ...")`.

## ex06_place_response

Start from `PlaceOrderResponse { command_id: Some(Id::from(command_id.value())), ..Default::default() }`. prost generates `set_status(...)` and `set_reject_reason(...)` for enum fields. `Uint128::from(quantity)` works for both quantity types (see `src/wire.rs`).

## ex07_market_place

`RestingOrder::new` takes the fields of `LimitGtc` in order. After `book.insert`, the status follows from two facts: `execution.fills().is_empty()` and `execution.resting_order_id()`. `total_filled_quantity()` already exists, and the quote is your ex04.
