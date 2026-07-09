# TDF Decoders C#

This folder contains a reusable C# library for decoding binary TDF data bytes into typed readings.

The library targets `netstandard2.1` and `net10.0`.

## Layout

- src/TdfDecoders: reusable library code
- generated/TdfDecoders.cs: generated TDF ID mappings, payload types, and payload decoders
- tests/TdfDecoders.Tests: xUnit tests for the decoder

## Decode a TDF block

Use `BlockDecoder.DecodeBlock` when you have raw TDF data bytes. It returns the readings decoded from the block, plus the first decode error if decoding stopped early.

Pass only the TDF bytes to this API. If your storage format prepends marker or framing bytes, strip those bytes before calling `DecodeBlock`.

```csharp
using TdfDecoders;

var block = new byte[]
{
    0x36, 0x00, 0x01, 87,
    0x00, 0x00,
};

var result = BlockDecoder.DecodeBlock(block);

foreach (var reading in result.Readings)
{
    Console.WriteLine($"{reading.Id}: {reading.Name}");

    foreach (var payload in reading.DecodedPayload)
    {
        if (payload is BatterySoc battery)
        {
            Console.WriteLine($"State of charge: {battery.Soc}%");
        }
    }

    if (reading.TryGetPayload<BatterySoc>(out var batterySoc))
    {
        Console.WriteLine($"First state of charge: {batterySoc.Soc}%");
    }
}

if (!result.Success)
{
    Console.WriteLine($"{result.Error!.Kind} at offset {result.Error.Offset}: {result.Error.Message}");
}
```

`DecodeBlock` also accepts `ReadOnlySpan<byte>` and `ReadOnlyMemory<byte>`.

Each `TdfReading` exposes:

- `Id`: numeric TDF ID
- `Name`: schema name when the ID is known
- `RawData`: read-only raw payload bytes for the whole decoded reading group
- `DecodedPayload`: generated typed payload objects when the ID and payload shape are known
- `TryGetPayload<T>` and `GetPayloads<T>`: typed helpers for decoded payloads
- `Timestamp`: optional `TdfTimestamp` metadata decoded from block flags
- `Period` and `Index`: optional metadata decoded from block flags

`TdfTimestamp` exposes the original fixed-point GPS timestamp via `EpochTime`, the split `GpsSeconds` and `FractionalTicks` components, and Unix conversion helpers:

```csharp
var timestamp = reading.Timestamp;
var unixTime = timestamp?.ToUnixTime();
var unixSeconds = timestamp?.ToUnixSeconds();
var dateTime = timestamp?.ToDateTimeOffset();
```

## Decode a known payload

If you already have the payload bytes for a known TDF ID, use the generated decoder directly:

```csharp
var battery = BatterySoc.Decode(new byte[] { 87 });

Console.WriteLine(battery.Soc);
```

For dynamic code, `TdfIdMappings` provides lookup helpers:

```csharp
var name = TdfIdMappings.GetName(54);
var type = TdfIdMappings.GetPayloadType(54);
var payload = TdfIdMappings.DecodePayload(54, new byte[] { 87 });
var typedPayload = TdfIdMappings.DecodePayload<BatterySoc>(54, new byte[] { 87 });

if (TdfIdMappings.TryDecodePayload<BatterySoc>(54, new byte[] { 87 }, out var batterySoc, out var exception))
{
    Console.WriteLine(batterySoc.Soc);
}
```

The generated payload decoders validate that the whole payload was consumed and throw when extra bytes remain.
Generated payload decoders also accept `ReadOnlySpan<byte>`.

## Test

```sh
dotnet test TdfDecoders.sln
```
