using System;

namespace TdfDecoders;

/// <summary>
/// Describes the first error encountered while decoding a TDF block.
/// </summary>
public sealed class TdfDecodeError
{
    /// <summary>
    /// Initializes a new instance of the <see cref="TdfDecodeError"/> class.
    /// </summary>
    /// <param name="kind">The reason decoding stopped.</param>
    /// <param name="offset">The byte offset where the error was detected.</param>
    /// <param name="id">The TDF identifier being decoded, when known.</param>
    /// <param name="message">A human-readable description of the error.</param>
    /// <param name="exception">The underlying exception, when the error was caused by one.</param>
    public TdfDecodeError(
        TdfDecodeErrorKind kind,
        int offset,
        int? id,
        string message,
        Exception? exception = null)
    {
        Kind = kind;
        Offset = offset;
        Id = id;
        Message = message;
        Exception = exception;
    }

    /// <summary>
    /// Gets the reason decoding stopped.
    /// </summary>
    public TdfDecodeErrorKind Kind { get; }

    /// <summary>
    /// Gets the byte offset where the error was detected.
    /// </summary>
    public int Offset { get; }

    /// <summary>
    /// Gets the TDF identifier being decoded, when known.
    /// </summary>
    public int? Id { get; }

    /// <summary>
    /// Gets a human-readable description of the error.
    /// </summary>
    public string Message { get; }

    /// <summary>
    /// Gets the underlying exception, when the error was caused by one.
    /// </summary>
    public Exception? Exception { get; }
}
