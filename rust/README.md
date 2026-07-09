# TDF Decoders Rust

This folder contains the Rust crate for decoding binary TDF data bytes into typed reading metadata and generated payload values.

The crate is named `tdf` and uses decoder code generated from `common/tdf.json`.

## Layout

- `src/lib.rs`: block decoding and `TdfReading` / `TdfBlockDecodeResult` types
- `src/generated/decoders.rs`: generated TDF ID mappings and payload decoders
- `src/time.rs`: TDF timestamp conversion helpers
- `tests`: reference tests that compare decoded fixtures with `common/test_data`

## Decode a TDF block

Use `tdf::block_decode` when you have raw TDF data bytes. It returns a `TdfBlockDecodeResult` containing all readings decoded before the first block-level error, plus `error` when decoding stopped early.

Pass only the TDF bytes to this API. If your storage format prepends marker or framing bytes, strip those bytes before calling `block_decode`.

```rust
use tdf::decoders::TdfValue;

fn main() -> tdf::TdfDecodeResult<()> {
    let block = [
        0x36, 0x00, 0x01, 87,
        0x00, 0x00,
    ];

    let result = tdf::block_decode(None, &block);

    for reading in &result.readings {
        println!("{}: {:?}", reading.id, reading.name);

        for payload in &reading.decoded_payload {
            println!("payload type: {}", payload.type_name);

            for field in &payload.fields {
                match &field.value {
                    TdfValue::UInt(value) => println!("{} = {}", field.name, value),
                    TdfValue::Int(value) => println!("{} = {}", field.name, value),
                    TdfValue::Float(value) => println!("{} = {}", field.name, value),
                    TdfValue::String(value) => println!("{} = {}", field.name, value),
                    other => println!("{} = {:?}", field.name, other),
                }
            }
        }
    }

    if let Some(error) = result.error {
        eprintln!("decode stopped early: {error}");
    }

    Ok(())
}
```

Pass `Some(remote_id)` as the first argument when the decoded readings should carry a remote device ID.

Each `TdfReading` exposes:

- `remote_id`: optional remote device ID supplied to `block_decode`
- `id`: numeric TDF ID
- `name`: schema name when the ID is known
- `timestamp`: optional fixed-point GPS timestamp decoded from block flags
- `period` and `index`: optional array metadata decoded from block flags
- `raw_data`: binary payload bytes passed to the generated TDF payload decoders
- `decoded_payload`: generated payload values when the ID and payload shape are known

`TdfBlockDecodeResult::success()` returns `true` when `error` is absent.

Payload fields are represented with `TdfValue`, which can hold integers, floats, strings, bytes, lists, and nested structs.

## Decode a known payload

If you already have the payload bytes for a known TDF ID, use the generated decoder directly:

```rust
let payload = tdf::decoders::tdf_decode_payload(54, 1, &[87])?;

if let Some(payload) = payload {
    println!("{}: {:?}", payload.type_name, payload.fields);
}
# Ok::<(), tdf::TdfDecodeError>(())
```

For dynamic code, the generated decoder module also provides TDF ID lookup helpers:

```rust
let name = tdf::decoders::tdf_known_name(54);
let payload = tdf::decoders::tdf_decode_payload(54, 1, &[87])?;
# Ok::<(), tdf::TdfDecodeError>(())
```

## Timestamps

`TdfReading::timestamp` stores the original fixed-point GPS timestamp as `i64`. Convert it with helpers from `tdf::time`:

```rust
if let Some(timestamp) = reading.timestamp {
    let (unix_seconds, unix_nanoseconds) = tdf::time::tdf_time_to_unix(timestamp);
    let unix_micros = tdf::time::tdf_time_to_unix_micros(timestamp);
    let datetime = tdf::time::tdf_time_to_datetime(timestamp);

    println!("{unix_seconds}.{unix_nanoseconds:09}");
    println!("{unix_micros}");
    println!("{datetime:?}");
}
```

## Test

From the repository root:

```sh
cargo test
```

Or from this crate:

```sh
cd rust
cargo test
```
