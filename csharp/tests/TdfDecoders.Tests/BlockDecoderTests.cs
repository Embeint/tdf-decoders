using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using TdfDecoders;
using Xunit;

namespace TdfDecoders.Tests;

public sealed class BlockDecoderTests
{
    [Fact]
    public void DecodeBlock_ReturnsParsedTdfsInSingleBlock()
    {
        var block = new byte[]
        {
            0xE7, 0x03, 0x02, 0xAA, 0xBB,
            0xE6, 0x03, 0x01, 0xCC,
            0x00, 0x00,
        };

        var readings = DecodeBlockSuccessfully(block);

        Assert.NotNull(readings);
        Assert.Equal(2, readings.Count);
        Assert.Equal(999, readings[0].Id);
        Assert.Null(readings[0].Name);
        Assert.Equal(new byte[] { 0xAA, 0xBB }, readings[0].RawData);
        Assert.Equal(998, readings[1].Id);
        Assert.Null(readings[1].Name);
        Assert.Equal(new byte[] { 0xCC }, readings[1].RawData);
    }

    [Fact]
    public void DecodeBlock_ReturnsSuccessfulResultWhenWholeBlockDecodes()
    {
        var block = new byte[]
        {
            0x36, 0x00, 0x01, 87,
            0x00, 0x00,
        };

        var result = BlockDecoder.DecodeBlock(block);

        Assert.True(result.Success);
        Assert.Null(result.Error);
        Assert.Single(result.Readings);
    }

    [Fact]
    public void DecodeBlock_TreatsTrailingPaddingBytesAsEndOfBlock()
    {
        foreach (var paddingByte in new byte[] { 0x00, 0xFF })
        {
            var block = new byte[]
            {
                0x36, 0x00, 0x01, 87,
                paddingByte, paddingByte, paddingByte, paddingByte, paddingByte,
            };

            var result = BlockDecoder.DecodeBlock(block);

            Assert.True(result.Success, result.Error?.Message);
            Assert.Null(result.Error);
            Assert.Single(result.Readings);
        }
    }

    [Fact]
    public void DecodeBlock_DoesNotTreatMixedTrailingPaddingBytesAsEndOfBlock()
    {
        var block = new byte[]
        {
            0x36, 0x00, 0x01, 87,
            0xFF, 0x00, 0xFF,
        };

        var result = BlockDecoder.DecodeBlock(block);

        Assert.False(result.Success);
        Assert.Single(result.Readings);
        Assert.NotNull(result.Error);
        Assert.Equal(TdfDecodeErrorKind.TruncatedPayload, result.Error!.Kind);
    }

    [Fact]
    public void DecodeBlock_TreatsZeroAndFfffHeadersAfterTdfAsEndOfBlock()
    {
        foreach (var terminator in new[] { new byte[] { 0x00, 0x00 }, new byte[] { 0xFF, 0xFF } })
        {
            var block = new byte[]
            {
                0x36, 0x00, 0x01, 87,
                terminator[0], terminator[1], 0xAA,
            };

            var result = BlockDecoder.DecodeBlock(block);

            Assert.True(result.Success, result.Error?.Message);
            Assert.Null(result.Error);
            Assert.Single(result.Readings);
        }
    }

    [Fact]
    public void DecodeBlock_ReturnsReadingsAndFirstErrorWhenDecodeStops()
    {
        var block = new byte[]
        {
            0x36, 0x00, 0x01, 87,
            0x36, 0x00, 0x02, 50,
        };

        var result = BlockDecoder.DecodeBlock(block);

        Assert.False(result.Success);
        Assert.NotNull(result.Error);
        var error = result.Error!;
        Assert.Equal(TdfDecodeErrorKind.TruncatedPayload, error.Kind);
        Assert.Equal(7, error.Offset);
        Assert.Equal(54, error.Id);
        var reading = Assert.Single(result.Readings);
        Assert.Equal(54, reading.Id);
        Assert.Equal(new byte[] { 87 }, reading.RawData);
    }

    [Fact]
    public void DecodeBlock_ReturnsPayloadDecodeErrorForKnownInvalidPayload()
    {
        var block = new byte[]
        {
            0x36, 0x00, 0x02, 87, 0x00,
            0x00, 0x00,
        };

        var result = BlockDecoder.DecodeBlock(block);

        Assert.False(result.Success);
        Assert.Empty(result.Readings);
        Assert.NotNull(result.Error);
        var error = result.Error!;
        Assert.Equal(TdfDecodeErrorKind.PayloadDecodeFailed, error.Kind);
        Assert.Equal(3, error.Offset);
        Assert.Equal(54, error.Id);
        Assert.NotNull(error.Exception);
    }

    [Fact]
    public void DecodeBlock_DecodesMarkerLikeHeaderBytesAsTdfData()
    {
        var block = new byte[] { 0x01, 0x02, 0x01, 0xAA, 0x00, 0x00 };

        var readings = DecodeBlockSuccessfully(block);

        Assert.Single(readings);
        Assert.Equal(513, readings[0].Id);
        Assert.Equal(new byte[] { 0xAA }, readings[0].RawData);
    }

    [Fact]
    public void TdfReading_StoresIdNameTimestampIndexAndRawData()
    {
        var rawData = new byte[] { 0xAA, 0xBB, 0xCC };
        var decodedPayload = new object[] { new object() };

        var timestamp = new TdfTimestamp(1_700_000_000UL);
        var reading = new TdfReading(1, rawData, timestamp, period: 0.25, index: 7, decodedPayload);

        Assert.Equal(1, reading.Id);
        Assert.Equal("ANNOUNCE", reading.Name);
        Assert.Equal(timestamp, reading.Timestamp);
        Assert.Equal(0.25, reading.Period);
        Assert.Equal(7, reading.Index);
        Assert.Equal(rawData, reading.RawData);
        Assert.IsNotType<byte[]>(reading.RawData);
        Assert.Same(decodedPayload, reading.DecodedPayload);
    }

    [Fact]
    public void TdfReading_CopiesRawData()
    {
        var rawData = new byte[] { 0xAA, 0xBB, 0xCC };

        var reading = new TdfReading(1, rawData);
        rawData[0] = 0x00;

        Assert.Equal(new byte[] { 0xAA, 0xBB, 0xCC }, reading.RawData);
    }

    [Fact]
    public void TdfTimestamp_ToUnixTime_ConvertsGpsEpochTicks()
    {
        var epochTime = (100UL << 16) | 32768UL;

        var unixTime = TdfTimestamp.ToUnixTime(epochTime);

        Assert.Equal(315_964_882, unixTime.Seconds);
        Assert.Equal(500_000_000u, unixTime.Nanoseconds);
    }

    [Fact]
    public void TdfTimestamp_ToUnixSeconds_ReturnsFractionalUnixSeconds()
    {
        var epochTime = (100UL << 16) | 32768UL;

        Assert.Equal(315_964_882.5, TdfTimestamp.ToUnixSeconds(epochTime));
    }

    [Fact]
    public void TdfTimestamp_ToDateTimeOffset_ReturnsUtcDateTime()
    {
        var epochTime = (100UL << 16) | 32768UL;
        var expected = DateTimeOffset.FromUnixTimeSeconds(315_964_882).AddTicks(5_000_000);

        Assert.Equal(expected, TdfTimestamp.ToDateTimeOffset(epochTime));
        Assert.Equal(expected, new TdfTimestamp(epochTime).ToDateTimeOffset());
    }

    [Fact]
    public void TdfTimestamp_ExposesEpochAndFractionalParts()
    {
        var timestamp = new TdfTimestamp((100UL << 16) | 32768UL);

        Assert.Equal((100UL << 16) | 32768UL, timestamp.EpochTime);
        Assert.Equal(100UL, timestamp.GpsSeconds);
        Assert.Equal(32768, timestamp.FractionalTicks);
        Assert.Equal((315_964_882, 500_000_000u), timestamp.ToUnixTime());
        Assert.Equal(DateTimeOffset.FromUnixTimeSeconds(315_964_882).AddTicks(5_000_000), timestamp.ToDateTimeOffset());
        Assert.Equal("315964882.5", timestamp.ToString());
    }

    [Fact]
    public void TdfTimestamp_UsesValueEquality()
    {
        var timestamp = new TdfTimestamp(123UL);
        var sameTimestamp = new TdfTimestamp(123UL);
        var differentTimestamp = new TdfTimestamp(456UL);

        Assert.Equal(timestamp, sameTimestamp);
        Assert.True(timestamp == sameTimestamp);
        Assert.True(timestamp != differentTimestamp);
        Assert.False(timestamp.Equals((object?)null));
    }

    [Fact]
    public void TdfReading_TimestampIsNullWhenAbsent()
    {
        var reading = new TdfReading(1, Array.Empty<byte>());

        Assert.Null(reading.Timestamp);
    }

    [Fact]
    public void DecodeBlock_DecodesExtendedRelativeTimestampAsSignedOffset()
    {
        var absoluteIdFlags = (ushort)(999 | 0x4000);
        var relativeIdFlags = (ushort)(998 | 0xC000);
        var absoluteEpochTime = (100UL << 16) | 32UL;
        var block = new byte[]
        {
            (byte)(absoluteIdFlags & 0xFF), (byte)(absoluteIdFlags >> 8), 0x01,
            100, 0x00, 0x00, 0x00, 32, 0x00,
            0xAA,
            (byte)(relativeIdFlags & 0xFF), (byte)(relativeIdFlags >> 8), 0x01,
            0xF0, 0xFF, 0xFF,
            0xBB,
            0x00, 0x00,
        };

        var readings = DecodeBlockSuccessfully(block);

        Assert.Equal(2, readings.Count);
        Assert.Equal(absoluteEpochTime, readings[0].Timestamp?.EpochTime);
        Assert.Equal(absoluteEpochTime - 16, readings[1].Timestamp?.EpochTime);
    }

    [Fact]
    public void DecodeBlock_DecodesKnownPayloadTypes()
    {
        var block = new byte[]
        {
            0x36, 0x00, 0x01, 87,
            0x00, 0x00,
        };

        var readings = DecodeBlockSuccessfully(block);

        var payload = Assert.IsType<BatterySoc>(Assert.Single(Assert.Single(readings).DecodedPayload));
        Assert.Equal(87, payload.Soc);
    }

    [Fact]
    public void DecodeBlock_ProcessesFixtureDataIn512ByteChunksWithoutErrors()
    {
        var fixtureName = "tdf_example.bin";
        var fixturePath = FindFixturePath(fixtureName);
        var bytes = File.ReadAllBytes(fixturePath);

        Assert.True(bytes.Length > 0, $"Fixture {fixtureName} should not be empty.");

        for (var offset = 0; offset < bytes.Length; offset += 512)
        {
            var length = Math.Min(512, bytes.Length - offset);
            var chunk = new byte[length];
            Array.Copy(bytes, offset, chunk, 0, length);

            if ((chunk[0] == 0x00 && chunk[1] == 0x00) || (chunk[0] == 0xFF && chunk[1] == 0xFF))
            {
                // Skip empty blocks
                continue;
            }

            var readings = DecodeBlockSuccessfully(TdfPayloadFromStorageFixture(chunk));
            Assert.NotNull(readings);

            if (offset == 0)
            {
                Assert.NotEmpty(readings);
            }
        }
    }

    [Fact]
    public void DecodeBlock_ExpandsDiffArrayForDiff16_8()
    {
        // header idFlags: id=999, flags=DiffArrayFlag, diffType=DIFF_16_8, diffCount=1
        var idFlags = (ushort)(999 | 0x2000);
        var header = new byte[] { (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x04, 0x41, 0x00, 0x00 };
        var payload = new byte[] { 0x01, 0x00, 0x02, 0x00 }; // two uint16 values: 1, 2
        var diffs = new byte[] { 0x01, 0xFF }; // delta values: +1, -1
        var block = new byte[header.Length + payload.Length + diffs.Length + 2];

        Array.Copy(header, 0, block, 0, header.Length);
        Array.Copy(payload, 0, block, header.Length, payload.Length);
        Array.Copy(diffs, 0, block, header.Length + payload.Length, diffs.Length);
        block[^2] = 0x00;
        block[^1] = 0x00;

        var readings = DecodeBlockSuccessfully(block);

        Assert.Single(readings);
        Assert.Equal(999, readings[0].Id);
        Assert.Equal(new byte[] { 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00 }, readings[0].RawData);
    }

    [Fact]
    public void DecodeBlock_ExpandsDiffArrayForDiff32_8()
    {
        // header idFlags: id=999, flags=DiffArrayFlag, diffType=DIFF_32_8, diffCount=1
        var idFlags = (ushort)(999 | 0x2000);
        var header = new byte[] { (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x04, 0x81, 0x00, 0x00 };
        var payload = new byte[] { 0x01, 0x00, 0x00, 0x00 }; // one uint32 value: 1
        var diffs = new byte[] { 0xFF }; // delta value: -1
        var block = new byte[header.Length + payload.Length + diffs.Length + 2];

        Array.Copy(header, 0, block, 0, header.Length);
        Array.Copy(payload, 0, block, header.Length, payload.Length);
        Array.Copy(diffs, 0, block, header.Length + payload.Length, diffs.Length);
        block[^2] = 0x00;
        block[^1] = 0x00;

        var readings = DecodeBlockSuccessfully(block);

        Assert.Single(readings);
        Assert.Equal(999, readings[0].Id);
        Assert.Equal(new byte[] { 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 }, readings[0].RawData);
    }

    [Fact]
    public void DecodeBlock_ExpandsDiffArrayForDiff32_16()
    {
        // header idFlags: id=999, flags=DiffArrayFlag, diffType=DIFF_32_16, diffCount=1
        var idFlags = (ushort)(999 | 0x2000);
        var header = new byte[] { (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x04, 0xC1, 0x00, 0x00 };
        var payload = new byte[] { 0x01, 0x00, 0x00, 0x00 }; // one uint32 value: 1
        var diffs = new byte[] { 0xFF, 0xFF }; // delta value: -1
        var block = new byte[header.Length + payload.Length + diffs.Length + 2];

        Array.Copy(header, 0, block, 0, header.Length);
        Array.Copy(payload, 0, block, header.Length, payload.Length);
        Array.Copy(diffs, 0, block, header.Length + payload.Length, diffs.Length);
        block[^2] = 0x00;
        block[^1] = 0x00;

        var readings = DecodeBlockSuccessfully(block);

        Assert.Single(readings);
        Assert.Equal(999, readings[0].Id);
        Assert.Equal(new byte[] { 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 }, readings[0].RawData);
    }

    [Fact]
    public void DecodeBlock_ProvidesPeriodForTimeArrayReadings()
    {
        var idFlags = (ushort)(999 | 0x1000);
        var block = new byte[]
        {
            (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x02,
            0x03, 0x00, 0x40,
            0xAA, 0xBB,
            0xCC, 0xDD,
            0xEE, 0xFF,
            0x00, 0x00,
        };

        var readings = DecodeBlockSuccessfully(block);

        Assert.Single(readings);
        Assert.Equal(0.25, readings[0].Period);
        Assert.Null(readings[0].Index);
        Assert.Equal(new byte[] { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF }, readings[0].RawData);
    }

    [Theory]
    [InlineData(0x1000)]
    [InlineData(0x3000)]
    public void DecodeBlock_RejectsZeroElementArrays(int arrayFlag)
    {
        var idFlags = (ushort)(999 | arrayFlag);
        var block = new byte[]
        {
            (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x01,
            0x00, 0x00, 0x00,
            0x00, 0x00,
        };

        var result = BlockDecoder.DecodeBlock(block);

        Assert.False(result.Success);
        Assert.Empty(result.Readings);
        Assert.NotNull(result.Error);
        Assert.Equal(TdfDecodeErrorKind.InvalidBlock, result.Error!.Kind);
        Assert.Equal(3, result.Error.Offset);
        Assert.Equal(999, result.Error.Id);
    }

    [Fact]
    public void DecodeBlock_AppliesPeriodScalingForTimeArrayReadings()
    {
        var idFlags = (ushort)(999 | 0x1000);
        var block = new byte[]
        {
            (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x01,
            0x02, 0x02, 0x80,
            0xAA,
            0xBB,
            0x00, 0x00,
        };

        var readings = DecodeBlockSuccessfully(block);

        Assert.Single(readings);
        Assert.Equal(0.25, readings[0].Period);
        Assert.Null(readings[0].Index);
        Assert.Equal(new byte[] { 0xAA, 0xBB }, readings[0].RawData);
    }

    [Fact]
    public void DecodeBlock_DecodesArrayPayloadsAsMultipleItems()
    {
        var idFlags = (ushort)(54 | 0x1000);
        var block = new byte[]
        {
            (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x01,
            0x03, 0x00, 0x40,
            10,
            20,
            30,
            0x00, 0x00,
        };

        var reading = Assert.Single(DecodeBlockSuccessfully(block));

        Assert.Equal(0.25, reading.Period);
        Assert.Equal(new byte[] { 10, 20, 30 }, reading.RawData);
        Assert.Collection(
            reading.DecodedPayload,
            first => Assert.Equal(10, Assert.IsType<BatterySoc>(first).Soc),
            second => Assert.Equal(20, Assert.IsType<BatterySoc>(second).Soc),
            third => Assert.Equal(30, Assert.IsType<BatterySoc>(third).Soc));
        Assert.True(reading.TryGetPayload<BatterySoc>(out var firstPayload));
        Assert.Equal(10, firstPayload.Soc);
        Assert.Equal(new byte[] { 10, 20, 30 }, reading.GetPayloads<BatterySoc>().Select(payload => payload.Soc));
    }

    [Fact]
    public void DecodeBlock_DecodesReadOnlySpan()
    {
        ReadOnlySpan<byte> block = stackalloc byte[]
        {
            0x36, 0x00, 0x01, 87,
            0x00, 0x00,
        };

        var result = BlockDecoder.DecodeBlock(block);

        Assert.True(result.Success, result.Error?.Message);
        Assert.Single(result.Readings);
        Assert.True(result.Readings[0].TryGetPayload<BatterySoc>(out var payload));
        Assert.Equal(87, payload.Soc);
    }

    [Fact]
    public void DecodeBlock_UsesArrayPeriodAsBaseIndexForIndexedArrays()
    {
        var idFlags = (ushort)(999 | 0x3000);
        var block = new byte[]
        {
            (byte)(idFlags & 0xFF), (byte)(idFlags >> 8), 0x01,
            0x02, 0x34, 0x12,
            0xAA,
            0xBB,
            0x00, 0x00,
        };

        var readings = DecodeBlockSuccessfully(block);

        Assert.Single(readings);
        Assert.Equal(0x1234 / 65536.0, readings[0].Period);
        Assert.Equal(0x1234, readings[0].Index);
        Assert.Equal(new byte[] { 0xAA, 0xBB }, readings[0].RawData);
    }

    [Fact]
    public void DecodeBlock_DecodesHighLevelPathFixture()
    {
        var fixturePath = FindFixturePath("tdf_example.bin");
        var bytes = File.ReadAllBytes(fixturePath);
        var readings = DecodeBlockSuccessfully(TdfPayloadFromStorageFixture(bytes));
        var absoluteEpochTime = (100UL << 16) | 32768UL;

        Assert.Equal(18, readings.Count);

        Assert.Equal(2001, readings[0].Id);
        Assert.Null(readings[0].Name);
        Assert.Null(readings[0].Timestamp);
        Assert.Equal(new byte[] { 0xAA, 0xBB, 0xCC }, readings[0].RawData);
        Assert.Empty(readings[0].DecodedPayload);

        Assert.Equal(54, readings[1].Id);
        Assert.Equal(absoluteEpochTime, readings[1].Timestamp?.EpochTime);
        Assert.Equal(66, Assert.IsType<BatterySoc>(Assert.Single(readings[1].DecodedPayload)).Soc);

        Assert.Equal(53, readings[2].Id);
        Assert.Equal(absoluteEpochTime + 256, readings[2].Timestamp?.EpochTime);
        Assert.Equal(4000, Assert.IsType<BatteryVoltage>(Assert.Single(readings[2].DecodedPayload)).Voltage);

        Assert.Equal(1000, readings[3].Id);
        Assert.Null(readings[3].Name);
        Assert.Equal(absoluteEpochTime + 272, readings[3].Timestamp?.EpochTime);
        Assert.Equal(new byte[] { 0x11, 0x22 }, readings[3].RawData);
        Assert.Empty(readings[3].DecodedPayload);

        Assert.Equal(54, readings[4].Id);
        Assert.Equal(0.25, readings[4].Period);
        Assert.Null(readings[4].Index);
        Assert.Equal(new byte[] { 10, 20, 30 }, readings[4].RawData);
        Assert.Equal(new byte[] { 10, 20, 30 }, readings[4].GetPayloads<BatterySoc>().Select(payload => payload.Soc));

        Assert.Equal(999, readings[5].Id);
        Assert.Equal(0.0, readings[5].Period);
        Assert.Null(readings[5].Index);
        Assert.Equal(new byte[] { 0x10, 0x00, 0x11, 0x00, 0x0F, 0x00 }, readings[5].RawData);
        Assert.Empty(readings[5].DecodedPayload);

        Assert.Equal(998, readings[6].Id);
        Assert.Equal(0.0, readings[6].Period);
        Assert.Null(readings[6].Index);
        Assert.Equal(
            new byte[] { 0x00, 0x00, 0x01, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00 },
            readings[6].RawData);
        Assert.Empty(readings[6].DecodedPayload);

        Assert.Equal(997, readings[7].Id);
        Assert.Equal(0.0, readings[7].Period);
        Assert.Null(readings[7].Index);
        Assert.Equal(
            new byte[] { 0x00, 0x00, 0x02, 0x00, 0x00, 0x01, 0x02, 0x00, 0x00, 0xFF, 0x01, 0x00 },
            readings[7].RawData);
        Assert.Empty(readings[7].DecodedPayload);

        Assert.Equal(54, readings[8].Id);
        Assert.Equal(0x1234 / 65536.0, readings[8].Period);
        Assert.Equal(0x1234, readings[8].Index);
        Assert.Equal(new byte[] { 55, 66 }, readings[8].RawData);
        Assert.Equal(new byte[] { 55, 66 }, readings[8].GetPayloads<BatterySoc>().Select(payload => payload.Soc));

        Assert.Equal(1001, readings[15].Id);
        Assert.Equal(absoluteEpochTime + 256, readings[15].Timestamp?.EpochTime);
        Assert.Equal(new byte[] { 0x33 }, readings[15].RawData);

        Assert.Equal(1002, readings[16].Id);
        Assert.Equal(absoluteEpochTime + 224, readings[16].Timestamp?.EpochTime);
        Assert.Equal(new byte[] { 0x44 }, readings[16].RawData);

        Assert.Equal(1003, readings[17].Id);
        Assert.Equal(absoluteEpochTime + 224, readings[17].Timestamp?.EpochTime);
        Assert.Equal(2.0, readings[17].Period);
        Assert.Null(readings[17].Index);
        Assert.Equal(new byte[] { 0x55, 0x66 }, readings[17].RawData);
    }

    [Fact]
    public void DecodeBlock_Returns18ReadingsForExampleFixture()
    {
        var fixtureName = "tdf_example.bin";
        var fixturePath = FindFixturePath(fixtureName);
        var bytes = File.ReadAllBytes(fixturePath);

        var totalReadings = 0;

        for (var offset = 0; offset < bytes.Length; offset += 512)
        {
            var length = Math.Min(512, bytes.Length - offset);
            var chunk = new byte[length];
            Array.Copy(bytes, offset, chunk, 0, length);

            if ((chunk[0] == 0x00 && chunk[1] == 0x00) || (chunk[0] == 0xFF && chunk[1] == 0xFF))
            {
                continue;
            }

            totalReadings += DecodeBlockSuccessfully(TdfPayloadFromStorageFixture(chunk)).Count;
        }

        Assert.Equal(18, totalReadings);
    }

    [Fact]
    public void DecodeBlock_DecodesTwoBlockFixtureWhenSplitInto512ByteBlocks()
    {
        var fixturePath = FindFixturePath("tdf_two_blocks.bin");
        var bytes = File.ReadAllBytes(fixturePath);
        var readings = new List<TdfReading>();

        Assert.Equal(1024, bytes.Length);

        for (var offset = 0; offset < bytes.Length; offset += 512)
        {
            var chunk = new byte[512];
            Array.Copy(bytes, offset, chunk, 0, chunk.Length);

            readings.AddRange(DecodeBlockSuccessfully(TdfPayloadFromStorageFixture(chunk)));
        }

        Assert.Equal(new[] { 999, 998, 997, 996 }, readings.Select(reading => reading.Id));
        Assert.Equal(new byte[] { 0xAA }, readings[0].RawData);
        Assert.Equal(new byte[] { 0xBB, 0xCC }, readings[1].RawData);
        Assert.Equal(new byte[] { 0xDD }, readings[2].RawData);
        Assert.Equal(new byte[] { 0x11, 0x22, 0x33 }, readings[3].RawData);
    }

    private static string FindFixturePath(string fixtureName)
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);

        while (directory is not null)
        {
            var candidate = Path.Combine(directory.FullName, "common", "test_data", fixtureName);
            if (File.Exists(candidate))
            {
                return candidate;
            }

            directory = directory.Parent;
        }

        throw new FileNotFoundException($"Could not find common/test_data/{fixtureName}.");
    }

    private static IReadOnlyList<TdfReading> DecodeBlockSuccessfully(byte[] block)
    {
        var result = BlockDecoder.DecodeBlock(block);

        Assert.True(result.Success, result.Error?.Message);
        Assert.Null(result.Error);
        return result.Readings;
    }

    private static byte[] TdfPayloadFromStorageFixture(byte[] bytes)
    {
        var payload = new byte[bytes.Length - 2];
        Array.Copy(bytes, 2, payload, 0, payload.Length);
        return payload;
    }
}
