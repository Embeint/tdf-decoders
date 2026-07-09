using System;
using System.Collections.Generic;

namespace TdfDecoders;

/// <summary>
/// Represents a decoded TDF block entry with its identifier, metadata, and raw payload bytes.
/// </summary>
public sealed class TdfReading
{
    /// <summary>
    /// Initializes a new instance of the <see cref="TdfReading"/> class.
    /// </summary>
    /// <param name="id">The numeric TDF identifier.</param>
    /// <param name="rawData">The raw payload bytes for the reading.</param>
    /// <param name="timestamp">The optional decoded timestamp.</param>
    /// <param name="period">The optional time period between array readings, in seconds.</param>
    /// <param name="index">The optional base array index associated with indexed-array readings.</param>
    /// <param name="decodedPayload">The optional typed payloads decoded from <paramref name="rawData"/>.</param>
    public TdfReading(
        int id,
        byte[] rawData,
        TdfTimestamp? timestamp = null,
        double? period = null,
        int? index = null,
        IReadOnlyList<object>? decodedPayload = null)
    {
        Id = id;
        RawData = Array.AsReadOnly((byte[])(rawData?.Clone() ?? Array.Empty<byte>()));
        Timestamp = timestamp;
        Period = period;
        Index = index;
        DecodedPayload = decodedPayload ?? Array.Empty<object>();
    }

    /// <summary>
    /// Gets the numeric TDF identifier.
    /// </summary>
    public int Id { get; }

    /// <summary>
    /// Gets the schema name for the TDF identifier, when known.
    /// </summary>
    public string? Name => TdfIdMappings.GetName(Id);

    /// <summary>
    /// Gets the decoded timestamp, when present.
    /// </summary>
    public TdfTimestamp? Timestamp { get; }

    /// <summary>
    /// Gets the time period between array readings in seconds, when present.
    /// </summary>
    public double? Period { get; }

    /// <summary>
    /// Gets the base index for indexed-array readings, when present.
    /// </summary>
    public int? Index { get; }

    /// <summary>
    /// Gets the raw payload bytes for the reading.
    /// </summary>
    public IReadOnlyList<byte> RawData { get; }

    /// <summary>
    /// Gets the typed payloads decoded from <see cref="RawData"/>, when the TDF ID is known and the payload is valid.
    /// </summary>
    public IReadOnlyList<object> DecodedPayload { get; }

    /// <summary>
    /// Attempts to get the first decoded payload assignable to <typeparamref name="T"/>.
    /// </summary>
    /// <typeparam name="T">The expected payload type.</typeparam>
    /// <param name="payload">The first matching payload, when one is present.</param>
    /// <returns><see langword="true"/> when a matching payload was found; otherwise, <see langword="false"/>.</returns>
    public bool TryGetPayload<T>(out T payload)
    {
        foreach (var item in DecodedPayload)
        {
            if (item is T typed)
            {
                payload = typed;
                return true;
            }
        }

        payload = default!;
        return false;
    }

    /// <summary>
    /// Gets all decoded payloads assignable to <typeparamref name="T"/>.
    /// </summary>
    /// <typeparam name="T">The expected payload type.</typeparam>
    /// <returns>The matching decoded payloads.</returns>
    public IReadOnlyList<T> GetPayloads<T>()
    {
        var payloads = new List<T>();
        foreach (var item in DecodedPayload)
        {
            if (item is T typed)
            {
                payloads.Add(typed);
            }
        }

        return payloads;
    }
}
