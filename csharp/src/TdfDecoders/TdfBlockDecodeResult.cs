using System;
using System.Collections.Generic;

namespace TdfDecoders;

/// <summary>
/// Represents the result of decoding a TDF block.
/// </summary>
public sealed class TdfBlockDecodeResult
{
    /// <summary>
    /// Initializes a new instance of the <see cref="TdfBlockDecodeResult"/> class.
    /// </summary>
    /// <param name="readings">The readings decoded before decoding stopped.</param>
    /// <param name="error">The first decode error, or <see langword="null"/> when decoding completed successfully.</param>
    public TdfBlockDecodeResult(IReadOnlyList<TdfReading>? readings, TdfDecodeError? error = null)
    {
        Readings = readings ?? Array.Empty<TdfReading>();
        Error = error;
    }

    /// <summary>
    /// Gets the readings decoded before decoding stopped.
    /// </summary>
    public IReadOnlyList<TdfReading> Readings { get; }

    /// <summary>
    /// Gets the first decode error, or <see langword="null"/> when decoding completed successfully.
    /// </summary>
    public TdfDecodeError? Error { get; }

    /// <summary>
    /// Gets a value indicating whether decoding completed without an error.
    /// </summary>
    public bool Success => Error is null;
}
