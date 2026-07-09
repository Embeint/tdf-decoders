using System;
using System.Collections.Generic;

namespace TdfDecoders;

/// <summary>
/// Decodes binary TDF blocks into individual raw TDF readings.
/// </summary>
public static class BlockDecoder
{
    private const ushort TerminatorHeader = 0x0000;
    private const ushort TerminatorHeaderAlt = 0xFFFF;
    private const ushort TimestampMask = 0xC000;
    private const ushort TimestampNone = 0x0000;
    private const ushort TimestampAbsolute = 0x4000;
    private const ushort TimestampRelative = 0x8000;
    private const ushort TimestampExtendedRelative = 0xC000;
    private const ushort ArrayMask = 0x3000;
    private const ushort DiffArrayFlag = 0x2000;
    private const ushort TimeArrayFlag = 0x1000;
    private const ushort IdxArrayFlag = 0x3000;
    private const int TimestampFractionalShift = 16;
    private const ushort PeriodScalingBit = 0x8000;
    private const ushort PeriodScalingValueMask = 0x7FFF;
    private const int PeriodScalingMultiplier = 8192;
    private const int Diff16_8 = 1;
    private const int Diff32_8 = 2;
    private const int Diff32_16 = 3;

    /// <summary>
    /// Decodes a binary TDF block into the readings contained in that block.
    /// </summary>
    /// <param name="block">The raw TDF block bytes.</param>
    /// <returns>The decoded readings and the first decode error, when one occurs.</returns>
    public static TdfBlockDecodeResult DecodeBlock(byte[] block)
    {
        var readings = new System.Collections.Generic.List<TdfReading>();

        if (block is null)
        {
            return Fail(
                readings,
                TdfDecodeErrorKind.InvalidBlock,
                0,
                null,
                "Block cannot be null.");
        }

        if (block.Length == 0)
        {
            return new TdfBlockDecodeResult(readings);
        }

        var offset = 0;
        ulong? globalEpochTime = null;

        while (offset < block.Length)
        {
            if (IsPadding(block, offset))
            {
                break;
            }

            if (offset + 2 > block.Length)
            {
                return Fail(
                    readings,
                    TdfDecodeErrorKind.TruncatedHeader,
                    offset,
                    null,
                    "Block ended before a complete TDF header could be read.");
            }

            var idFlags = ReadUInt16LittleEndian(block, offset);

            if (idFlags == TerminatorHeader || idFlags == TerminatorHeaderAlt)
            {
                break;
            }

            if (offset + 3 > block.Length)
            {
                return Fail(
                    readings,
                    TdfDecodeErrorKind.TruncatedHeader,
                    offset,
                    ReadId(idFlags),
                    "Block ended before a complete TDF header could be read.");
            }

            var payloadLength = block[offset + 2];
            var cursor = offset + 3;
            int? index = null;
            double? period = null;
            var arrayHeaderNum = 0;

            var timeFlags = idFlags & TimestampMask;
            if (timeFlags == TimestampAbsolute)
            {
                if (cursor + 6 > block.Length)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.TruncatedTimestamp,
                        cursor,
                        ReadId(idFlags),
                        "Block ended before an absolute timestamp could be read.");
                }

                var seconds = ReadUInt32LittleEndian(block, cursor);
                var subseconds = ReadUInt16LittleEndian(block, cursor + 4);
                globalEpochTime = ((ulong)seconds << TimestampFractionalShift) | subseconds;
                cursor += 6;
            }
            else if (timeFlags == TimestampRelative)
            {
                if (!globalEpochTime.HasValue)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.MissingAbsoluteTimestamp,
                        cursor,
                        ReadId(idFlags),
                        "Relative timestamp encountered before an absolute timestamp.");
                }

                if (cursor + 2 > block.Length)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.TruncatedTimestamp,
                        cursor,
                        ReadId(idFlags),
                        "Block ended before a relative timestamp could be read.");
                }

                var offsetValue = ReadUInt16LittleEndian(block, cursor);
                globalEpochTime = globalEpochTime.Value + offsetValue;
                cursor += 2;
            }
            else if (timeFlags == TimestampExtendedRelative)
            {
                if (!globalEpochTime.HasValue)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.MissingAbsoluteTimestamp,
                        cursor,
                        ReadId(idFlags),
                        "Extended relative timestamp encountered before an absolute timestamp.");
                }

                if (cursor + 3 > block.Length)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.TruncatedTimestamp,
                        cursor,
                        ReadId(idFlags),
                        "Block ended before an extended relative timestamp could be read.");
                }

                var offsetValue = ReadInt24LittleEndian(block, cursor);
                if (!TryAddTimestampOffset(globalEpochTime.Value, offsetValue, out var updatedEpochTime))
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.InvalidBlock,
                        cursor,
                        ReadId(idFlags),
                        "Extended relative timestamp offset moved outside the supported timestamp range.");
                }

                globalEpochTime = updatedEpochTime;
                cursor += 3;
            }

            var arrayFlags = idFlags & ArrayMask;
            if (arrayFlags == DiffArrayFlag || arrayFlags == TimeArrayFlag || arrayFlags == IdxArrayFlag)
            {
                if (cursor + 3 > block.Length)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.TruncatedArrayHeader,
                        cursor,
                        ReadId(idFlags),
                        "Block ended before a complete array header could be read.");
                }

                arrayHeaderNum = block[cursor];
                var arrayPeriod = ReadUInt16LittleEndian(block, cursor + 1);
                if (arrayHeaderNum == 0 && arrayFlags is TimeArrayFlag or IdxArrayFlag)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.InvalidBlock,
                        cursor,
                        ReadId(idFlags),
                        "Array header declares zero elements.");
                }

                if (arrayFlags == IdxArrayFlag)
                {
                    index = arrayPeriod;
                    period = arrayPeriod / 65536.0;
                }
                else
                {
                    period = DecodeArrayPeriod(arrayPeriod);
                }
                cursor += 3;
            }

            var payloadStart = cursor;
            var payloadEnd = cursor + payloadLength;
            if (payloadEnd > block.Length)
            {
                return Fail(
                    readings,
                    TdfDecodeErrorKind.TruncatedPayload,
                    payloadStart,
                    ReadId(idFlags),
                    "Block ended before the payload could be read.");
            }

            var payloadDataForDecode = CopyBytes(block, payloadStart, payloadLength);

            if (arrayFlags == DiffArrayFlag)
            {
                var diffType = (arrayHeaderNum >> 6) & 0x03;
                var diffCount = arrayHeaderNum & 0x3F;
                var diffPayloadBytes = diffType switch
                {
                    Diff16_8 => payloadLength / 2,
                    Diff32_8 => payloadLength / 4,
                    Diff32_16 => payloadLength / 2,
                    _ => 0,
                };

                if (diffType is not Diff16_8 and not Diff32_8 and not Diff32_16)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.InvalidDiffArray,
                        payloadStart,
                        ReadId(idFlags),
                        "Diff array uses an unsupported diff type.");
                }

                if (!IsValidDiffBasePayloadLength(payloadLength, diffType))
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.InvalidDiffArray,
                        payloadStart,
                        ReadId(idFlags),
                        "Diff array base payload length is not valid for the diff type.");
                }

                var diffArrayEnd = payloadStart + payloadLength + (diffCount * diffPayloadBytes);
                if (diffArrayEnd > block.Length)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.InvalidDiffArray,
                        payloadStart + payloadLength,
                        ReadId(idFlags),
                        "Block ended before the diff array payload could be read.");
                }

                payloadDataForDecode = ExpandDiffArray(block, payloadStart, payloadLength, diffType, diffCount);
                cursor += payloadLength + (diffCount * diffPayloadBytes);
            }
            else if (arrayFlags == TimeArrayFlag || arrayFlags == IdxArrayFlag)
            {
                var arrayPayloadLength = arrayHeaderNum * payloadLength;
                var arrayPayloadEnd = payloadStart + arrayPayloadLength;
                if (arrayPayloadEnd > block.Length)
                {
                    return Fail(
                        readings,
                        TdfDecodeErrorKind.TruncatedPayload,
                        payloadStart,
                        ReadId(idFlags),
                        "Block ended before the array payload could be read.");
                }

                payloadDataForDecode = new byte[arrayPayloadLength];
                Array.Copy(block, payloadStart, payloadDataForDecode, 0, arrayPayloadLength);
                cursor += arrayPayloadLength;
            }
            else
            {
                cursor += payloadLength;
            }

            if (cursor > block.Length)
            {
                return Fail(
                    readings,
                    TdfDecodeErrorKind.TruncatedPayload,
                    payloadStart,
                    ReadId(idFlags),
                    "Block ended before the payload could be read.");
            }

            var id = ReadId(idFlags);
            if (!TryDecodePayloads(id, payloadDataForDecode, payloadLength, payloadStart, out var decodedPayload, out var error))
            {
                return new TdfBlockDecodeResult(readings, error);
            }

            TdfTimestamp? timestamp = globalEpochTime.HasValue ? new TdfTimestamp(globalEpochTime.Value) : null;
            var rawData = payloadDataForDecode;
            readings.Add(new TdfReading(id, rawData, timestamp, period, index, decodedPayload));

            offset = cursor;
        }

        return new TdfBlockDecodeResult(readings);
    }

    /// <summary>
    /// Decodes a binary TDF block into the readings contained in that block.
    /// </summary>
    /// <param name="block">The raw TDF block bytes.</param>
    /// <returns>The decoded readings and the first decode error, when one occurs.</returns>
    public static TdfBlockDecodeResult DecodeBlock(ReadOnlySpan<byte> block)
    {
        return DecodeBlock(block.ToArray());
    }

    /// <summary>
    /// Decodes a binary TDF block into the readings contained in that block.
    /// </summary>
    /// <param name="block">The raw TDF block bytes.</param>
    /// <returns>The decoded readings and the first decode error, when one occurs.</returns>
    public static TdfBlockDecodeResult DecodeBlock(ReadOnlyMemory<byte> block)
    {
        return DecodeBlock(block.Span);
    }

    private static ushort ReadUInt16LittleEndian(byte[] buffer, int offset)
    {
        return (ushort)(buffer[offset] | (buffer[offset + 1] << 8));
    }

    private static uint ReadUInt32LittleEndian(byte[] buffer, int offset)
    {
        return (uint)(buffer[offset] | (buffer[offset + 1] << 8) | (buffer[offset + 2] << 16) | (buffer[offset + 3] << 24));
    }

    private static int ReadInt24LittleEndian(byte[] buffer, int offset)
    {
        var value = buffer[offset] | (buffer[offset + 1] << 8) | (buffer[offset + 2] << 16);
        if ((value & 0x800000) != 0)
        {
            value |= unchecked((int)0xFF000000);
        }

        return value;
    }

    private static bool TryAddTimestampOffset(ulong epochTime, int offset, out ulong result)
    {
        if (offset >= 0)
        {
            result = epochTime + (uint)offset;
            return result >= epochTime;
        }

        var magnitude = (uint)-offset;
        if (epochTime < magnitude)
        {
            result = 0;
            return false;
        }

        result = epochTime - magnitude;
        return true;
    }

    private static double DecodeArrayPeriod(ushort encodedPeriod)
    {
        var period = encodedPeriod & PeriodScalingValueMask;
        if ((encodedPeriod & PeriodScalingBit) != 0)
        {
            period *= PeriodScalingMultiplier;
        }

        return period / 65536.0;
    }

    private static byte[] CopyBytes(byte[] buffer, int offset, int length)
    {
        var bytes = new byte[length];
        Array.Copy(buffer, offset, bytes, 0, length);
        return bytes;
    }

    private static bool IsPadding(byte[] buffer, int offset)
    {
        var paddingByte = buffer[offset];
        if (paddingByte is not 0x00 and not 0xFF)
        {
            return false;
        }

        for (var index = offset; index < buffer.Length; index++)
        {
            if (buffer[index] != paddingByte)
            {
                return false;
            }
        }

        return true;
    }

    private static TdfBlockDecodeResult Fail(
        IReadOnlyList<TdfReading> readings,
        TdfDecodeErrorKind kind,
        int offset,
        int? id,
        string message,
        Exception? exception = null)
    {
        return new TdfBlockDecodeResult(readings, new TdfDecodeError(kind, offset, id, message, exception));
    }

    private static bool IsValidDiffBasePayloadLength(int payloadLength, int diffType)
    {
        var baseElementSize = diffType switch
        {
            Diff16_8 => 2,
            Diff32_8 => 4,
            Diff32_16 => 4,
            _ => 0,
        };

        return baseElementSize > 0 && payloadLength % baseElementSize == 0;
    }

    private static byte[] ExpandDiffArray(byte[] block, int payloadStart, int payloadLength, int diffType, int diffCount)
    {
        var expanded = new byte[payloadLength * (diffCount + 1)];
        Array.Copy(block, payloadStart, expanded, 0, payloadLength);

        if (payloadLength == 0 || diffCount == 0)
        {
            return expanded;
        }

        var baseElementSize = diffType switch
        {
            Diff16_8 => 2,
            Diff32_8 => 4,
            Diff32_16 => 4,
            _ => 0,
        };

        var diffElementSize = diffType switch
        {
            Diff16_8 => 1,
            Diff32_8 => 1,
            Diff32_16 => 2,
            _ => 0,
        };

        if (baseElementSize == 0 || diffElementSize == 0)
        {
            return expanded;
        }

        var numFields = payloadLength / baseElementSize;
        var diffPayloadBytes = numFields * diffElementSize;
        var diffOffsetStart = payloadStart + payloadLength;

        for (var diffIndex = 0; diffIndex < diffCount; diffIndex++)
        {
            var sourceOffset = diffOffsetStart + diffIndex * diffPayloadBytes;
            var targetOffset = (diffIndex + 1) * payloadLength;
            var previousOffset = diffIndex * payloadLength;

            if (diffType == Diff16_8)
            {
                for (var fieldIndex = 0; fieldIndex < numFields; fieldIndex++)
                {
                    var currentOffset = previousOffset + fieldIndex * 2;
                    var previousValue = ReadUInt16LittleEndian(expanded, currentOffset);
                    var delta = (sbyte)block[sourceOffset + fieldIndex];
                    WriteUInt16LittleEndian(expanded, targetOffset + fieldIndex * 2, (ushort)(previousValue + delta));
                }
            }
            else if (diffType == Diff32_8)
            {
                for (var fieldIndex = 0; fieldIndex < numFields; fieldIndex++)
                {
                    var currentOffset = previousOffset + fieldIndex * 4;
                    var previousValue = ReadUInt32LittleEndian(expanded, currentOffset);
                    var delta = (sbyte)block[sourceOffset + fieldIndex];
                    WriteUInt32LittleEndian(expanded, targetOffset + fieldIndex * 4, (uint)((long)previousValue + delta));
                }
            }
            else
            {
                for (var fieldIndex = 0; fieldIndex < numFields; fieldIndex++)
                {
                    var currentOffset = previousOffset + fieldIndex * 4;
                    var previousValue = ReadUInt32LittleEndian(expanded, currentOffset);
                    var delta = ReadInt16LittleEndian(block, sourceOffset + fieldIndex * 2);
                    WriteUInt32LittleEndian(expanded, targetOffset + fieldIndex * 4, (uint)((long)previousValue + delta));
                }
            }
        }

        return expanded;
    }

    private static short ReadInt16LittleEndian(byte[] buffer, int offset)
    {
        return (short)(buffer[offset] | (buffer[offset + 1] << 8));
    }

    private static void WriteUInt16LittleEndian(byte[] buffer, int offset, ushort value)
    {
        buffer[offset] = (byte)(value & 0xFF);
        buffer[offset + 1] = (byte)(value >> 8);
    }

    private static void WriteUInt32LittleEndian(byte[] buffer, int offset, uint value)
    {
        buffer[offset] = (byte)(value & 0xFF);
        buffer[offset + 1] = (byte)((value >> 8) & 0xFF);
        buffer[offset + 2] = (byte)((value >> 16) & 0xFF);
        buffer[offset + 3] = (byte)((value >> 24) & 0xFF);
    }

    private static int ReadId(ushort idFlags)
    {
        return idFlags & 0x0FFF;
    }

    private static bool TryDecodePayloads(
        int id,
        byte[] payloadData,
        int elementLength,
        int payloadOffset,
        out IReadOnlyList<object> decodedPayloads,
        out TdfDecodeError? error)
    {
        decodedPayloads = Array.Empty<object>();
        error = null;

        if (payloadData.Length == 0)
        {
            return true;
        }

        if (elementLength <= 0 || payloadData.Length == elementLength)
        {
            if (!TryDecodePayload(id, payloadData, payloadOffset, out var decoded, out error))
            {
                return false;
            }

            decodedPayloads = decoded is null ? Array.Empty<object>() : new[] { decoded };
            return true;
        }

        if (payloadData.Length % elementLength != 0)
        {
            error = new TdfDecodeError(
                TdfDecodeErrorKind.PayloadDecodeFailed,
                payloadOffset,
                id,
                "Payload length is not divisible by the element length.");
            return false;
        }

        var payloads = new List<object>();
        for (var offset = 0; offset < payloadData.Length; offset += elementLength)
        {
            var payload = new byte[elementLength];
            Array.Copy(payloadData, offset, payload, 0, elementLength);

            if (!TryDecodePayload(id, payload, payloadOffset + offset, out var decoded, out error))
            {
                return false;
            }

            if (decoded is not null)
            {
                payloads.Add(decoded);
            }
        }

        decodedPayloads = payloads;
        return true;
    }

    private static bool TryDecodePayload(
        int id,
        byte[] payload,
        int payloadOffset,
        out object? decodedPayload,
        out TdfDecodeError? error)
    {
        decodedPayload = null;
        error = null;

        if (TdfIdMappings.GetPayloadType(id) is null)
        {
            return true;
        }

        try
        {
            decodedPayload = TdfIdMappings.DecodePayload(id, payload);
            return true;
        }
        catch (ArgumentException exception)
        {
            error = new TdfDecodeError(
                TdfDecodeErrorKind.PayloadDecodeFailed,
                payloadOffset,
                id,
                "Payload could not be decoded for the known TDF ID.",
                exception);
            return false;
        }
    }
}
