namespace TdfDecoders;

/// <summary>
/// Identifies the reason a TDF block decode stopped before successfully consuming the block.
/// </summary>
public enum TdfDecodeErrorKind
{
    /// <summary>
    /// The provided block is not valid input.
    /// </summary>
    InvalidBlock,

    /// <summary>
    /// The block ended before a complete TDF header could be read.
    /// </summary>
    TruncatedHeader,

    /// <summary>
    /// The block ended before a complete timestamp could be read.
    /// </summary>
    TruncatedTimestamp,

    /// <summary>
    /// A relative timestamp was encountered before an absolute timestamp established the epoch.
    /// </summary>
    MissingAbsoluteTimestamp,

    /// <summary>
    /// The block ended before a complete array header could be read.
    /// </summary>
    TruncatedArrayHeader,

    /// <summary>
    /// The block ended before a complete payload could be read.
    /// </summary>
    TruncatedPayload,

    /// <summary>
    /// A diff array header or payload was not valid.
    /// </summary>
    InvalidDiffArray,

    /// <summary>
    /// A known TDF payload could not be decoded.
    /// </summary>
    PayloadDecodeFailed
}
