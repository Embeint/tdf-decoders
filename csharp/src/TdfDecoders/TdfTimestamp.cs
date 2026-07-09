using System;

namespace TdfDecoders;

/// <summary>
/// Represents a TDF timestamp encoded as fixed-point GPS seconds.
/// </summary>
public readonly struct TdfTimestamp : IEquatable<TdfTimestamp>
{
    private const long GpsUnixOffsetSecondsBase = 315_964_800;
    private const long GpsUnixOffsetSecondsLeap = 18;
    private const int FractionalShift = 16;
    private const ulong FractionalMask = 0xFFFF;
    private const ulong TicksPerSecond = 65_536;

    /// <summary>
    /// Initializes a new instance of the <see cref="TdfTimestamp"/> class.
    /// </summary>
    /// <param name="epochTime">The TDF timestamp in fixed-point GPS seconds.</param>
    public TdfTimestamp(ulong epochTime)
    {
        EpochTime = epochTime;
    }

    /// <summary>
    /// Gets the TDF timestamp in fixed-point GPS seconds.
    /// </summary>
    public ulong EpochTime { get; }

    /// <summary>
    /// Gets the whole GPS seconds component.
    /// </summary>
    public ulong GpsSeconds => EpochTime >> FractionalShift;

    /// <summary>
    /// Gets the fractional GPS second component, where 65,536 ticks represent one second.
    /// </summary>
    public ushort FractionalTicks => (ushort)(EpochTime & FractionalMask);

    /// <summary>
    /// Converts a TDF epoch timestamp to Unix time seconds and nanoseconds.
    /// </summary>
    /// <remarks>
    /// This conversion applies the GPS-to-Unix offset using the current 18-second GPS-UTC leap-second delta.
    /// Update <c>GpsUnixOffsetSecondsLeap</c> if the leap-second table changes. This offset has been correct
    /// since 2017 and is expected to remain valid until at least 2035.
    /// </remarks>
    /// <param name="epochTime">The TDF timestamp in fixed-point GPS seconds.</param>
    /// <returns>The Unix timestamp seconds and nanosecond fraction.</returns>
    public static (long Seconds, uint Nanoseconds) ToUnixTime(ulong epochTime)
    {
        var seconds = checked((long)(epochTime >> FractionalShift)
            + GpsUnixOffsetSecondsBase
            - GpsUnixOffsetSecondsLeap);
        var nanoseconds = (uint)((1_000_000_000UL * (epochTime & FractionalMask)) / TicksPerSecond);

        return (seconds, nanoseconds);
    }

    /// <summary>
    /// Converts a TDF epoch timestamp to Unix time seconds as a floating-point value.
    /// </summary>
    /// <param name="epochTime">The TDF timestamp in fixed-point GPS seconds.</param>
    /// <returns>The Unix timestamp in seconds, including the fractional component.</returns>
    public static double ToUnixSeconds(ulong epochTime)
    {
        var unixTime = ToUnixTime(epochTime);

        return unixTime.Seconds + (unixTime.Nanoseconds / 1_000_000_000.0);
    }

    /// <summary>
    /// Converts a TDF epoch timestamp to a UTC <see cref="DateTimeOffset"/>.
    /// </summary>
    /// <param name="epochTime">The TDF timestamp in fixed-point GPS seconds.</param>
    /// <returns>The UTC date and time represented by the timestamp.</returns>
    public static DateTimeOffset ToDateTimeOffset(ulong epochTime)
    {
        var unixTime = ToUnixTime(epochTime);

        return DateTimeOffset.FromUnixTimeSeconds(unixTime.Seconds).AddTicks(unixTime.Nanoseconds / 100);
    }

    /// <summary>
    /// Converts this timestamp to Unix time seconds and nanoseconds.
    /// </summary>
    /// <returns>The Unix timestamp seconds and nanosecond fraction.</returns>
    public (long Seconds, uint Nanoseconds) ToUnixTime()
    {
        return ToUnixTime(EpochTime);
    }

    /// <summary>
    /// Converts this timestamp to Unix time seconds as a floating-point value.
    /// </summary>
    /// <returns>The Unix timestamp in seconds, including the fractional component.</returns>
    public double ToUnixSeconds()
    {
        return ToUnixSeconds(EpochTime);
    }

    /// <summary>
    /// Converts this timestamp to a UTC <see cref="DateTimeOffset"/>.
    /// </summary>
    /// <returns>The UTC date and time represented by this timestamp.</returns>
    public DateTimeOffset ToDateTimeOffset()
    {
        return ToDateTimeOffset(EpochTime);
    }

    /// <inheritdoc />
    public bool Equals(TdfTimestamp other)
    {
        return EpochTime == other.EpochTime;
    }

    /// <inheritdoc />
    public override bool Equals(object? obj)
    {
        return obj is TdfTimestamp other && Equals(other);
    }

    /// <inheritdoc />
    public override int GetHashCode()
    {
        return EpochTime.GetHashCode();
    }

    /// <summary>
    /// Determines whether two timestamps represent the same fixed-point GPS time.
    /// </summary>
    /// <param name="left">The first timestamp.</param>
    /// <param name="right">The second timestamp.</param>
    /// <returns><see langword="true"/> when both timestamps have the same epoch time; otherwise, <see langword="false"/>.</returns>
    public static bool operator ==(TdfTimestamp left, TdfTimestamp right)
    {
        return left.Equals(right);
    }

    /// <summary>
    /// Determines whether two timestamps represent different fixed-point GPS times.
    /// </summary>
    /// <param name="left">The first timestamp.</param>
    /// <param name="right">The second timestamp.</param>
    /// <returns><see langword="true"/> when the timestamps have different epoch times; otherwise, <see langword="false"/>.</returns>
    public static bool operator !=(TdfTimestamp left, TdfTimestamp right)
    {
        return !left.Equals(right);
    }

    /// <inheritdoc />
    public override string ToString()
    {
        return ToUnixSeconds().ToString("R", System.Globalization.CultureInfo.InvariantCulture);
    }
}
