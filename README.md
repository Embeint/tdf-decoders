# TDF Decoders

Shared tooling and language implementations for decoding TDF binary data.

The repository keeps the TDF schema in one place and uses it to generate decoder code for language-specific libraries.

## Layout

- `common/tdf.json`: shared TDF definitions
- `common/scripts/generate_decoders.py`: generator for language-specific decoder code
- `common/test_data`: binary fixtures used by tests
- `rust`: Rust decoder crate and tests
- `csharp`: C# decoder library and tests

## Rust

The Rust crate decodes raw TDF data bytes into `TdfReading` values with generated dynamic payload fields.

```sh
cargo test
```

See `rust/README.md` for Rust API examples.

## C#

The C# library decodes raw TDF data bytes into `TdfReading` values and generated typed payloads.

```sh
cd csharp
dotnet test TdfDecoders.sln
```

See `csharp/README.md` for C# API examples.

## Regenerate Decoders

After changing `common/tdf.json`, regenerate the generated decoder sources:

```sh
python3 common/scripts/generate_decoders.py
```

To include downstream extension definitions, pass the extension JSON file:

```sh
python3 common/scripts/generate_decoders.py common/test_data/extension_definitions.json
```

Then run the relevant tests.

### Generating into a downstream project

Downstream projects that maintain their own extension definitions can generate
decoder sources straight into their own tree, leaving this repository untouched:

```sh
python3 common/scripts/generate_decoders.py path/to/extensions.json \
    --language csharp \
    --csharp-output ../my-project/src/Generated/TdfDecoders.cs \
    --csharp-namespace My.Project.Tdf
```

- `--language {all,csharp,rust}` selects which sources to generate. Restricting
  to `csharp` skips the Rust output and its `rustfmt` invocation.
- `--csharp-output` and `--rust-output` set the output file paths. Missing parent
  directories are created. Both default to the in-repo generated sources.
- `--csharp-namespace` sets the namespace of the generated C# source, so a
  downstream copy does not collide with this repository's `TdfDecoders` types.
  Defaults to `TdfDecoders`.
