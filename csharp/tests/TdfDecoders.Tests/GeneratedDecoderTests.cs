using System;
using TdfDecoders;
using Xunit;

namespace TdfDecoders.Tests;

public sealed class TdfIdMappingsTests
{
    [Fact]
    public void TdfIdMappings_ReturnsRegisteredNames()
    {
        Assert.Equal("ANNOUNCE", TdfIdMappings.GetName(1));
        Assert.Equal("BATTERY_STATE", TdfIdMappings.GetName(2));
        Assert.Null(TdfIdMappings.GetName(999999));
    }

    [Fact]
    public void TdfIdMappings_ReturnsPayloadTypes()
    {
        Assert.Equal(typeof(Announce), TdfIdMappings.GetPayloadType(1));
        Assert.Equal(typeof(BatterySoc), TdfIdMappings.GetPayloadType(54));
        Assert.Null(TdfIdMappings.GetPayloadType(999999));
    }

    [Fact]
    public void TdfIdMappings_DecodesPayloads()
    {
        var decoded = Assert.IsType<BatterySoc>(TdfIdMappings.DecodePayload(54, new byte[] { 87 }));

        Assert.Equal(87, decoded.Soc);
        Assert.Null(TdfIdMappings.DecodePayload(999999, Array.Empty<byte>()));
    }

    [Fact]
    public void TdfIdMappings_DecodesPayloadsWithTypedHelper()
    {
        var decoded = TdfIdMappings.DecodePayload<BatterySoc>(54, new byte[] { 87 });

        Assert.NotNull(decoded);
        Assert.Equal(87, decoded.Soc);
        Assert.Null(TdfIdMappings.DecodePayload<BatterySoc>(999999, Array.Empty<byte>()));
        Assert.Null(TdfIdMappings.DecodePayload<BatteryVoltage>(54, new byte[] { 87 }));
    }

    [Fact]
    public void TdfIdMappings_TryDecodePayloadReturnsExceptionForInvalidPayload()
    {
        var success = TdfIdMappings.TryDecodePayload<BatterySoc>(
            54,
            new byte[] { 87, 0 },
            out var decoded,
            out var exception);

        Assert.False(success);
        Assert.Null(decoded);
        Assert.IsType<ArgumentException>(exception);
    }

    [Fact]
    public void GeneratedTdfDecoder_DecodesReadOnlySpan()
    {
        ReadOnlySpan<byte> payload = stackalloc byte[] { 87 };

        var decoded = BatterySoc.Decode(payload);

        Assert.Equal(87, decoded.Soc);
    }

    [Fact]
    public void TdfIdMappings_DecodesReadOnlySpanPayloads()
    {
        ReadOnlySpan<byte> payload = stackalloc byte[] { 87 };

        var decoded = TdfIdMappings.DecodePayload<BatterySoc>(54, payload);

        Assert.NotNull(decoded);
        Assert.Equal(87, decoded.Soc);
        Assert.True(TdfIdMappings.TryDecodePayload<BatterySoc>(54, payload, out var tryDecoded, out var exception));
        Assert.Null(exception);
        Assert.Equal(87, tryDecoded!.Soc);
    }

    [Fact]
    public void GeneratedStructDecoder_DecodesLittleEndianFields()
    {
        var sample = TdfStructXyz16bit.Decode(new byte[] { 0x34, 0x12, 0xFE, 0xFF, 0x00, 0x80 });

        Assert.Equal(0x1234, sample.X);
        Assert.Equal(-2, sample.Y);
        Assert.Equal(short.MinValue, sample.Z);
    }

    [Fact]
    public void GeneratedTdfDecoder_DecodesNestedStructs()
    {
        var announce = Announce.Decode(new byte[]
        {
            0x04, 0x03, 0x02, 0x01,
            0x01,
            0x02,
            0x04, 0x03,
            0x08, 0x07, 0x06, 0x05,
            0x0C, 0x0B, 0x0A, 0x09,
            0x10, 0x0F, 0x0E, 0x0D,
            0x14, 0x13, 0x12, 0x11,
            0x16, 0x15,
            0x17,
        });

        Assert.Equal(0x01020304u, announce.Application);
        Assert.Equal(1, announce.Version.Major);
        Assert.Equal(2, announce.Version.Minor);
        Assert.Equal(0x0304, announce.Version.Revision);
        Assert.Equal(0x05060708u, announce.Version.BuildNum);
        Assert.Equal(0x090A0B0Cu, announce.KvCrc);
        Assert.Equal(0x0D0E0F10u, announce.Blocks);
        Assert.Equal(0x11121314u, announce.Uptime);
        Assert.Equal(0x1516, announce.Reboots);
        Assert.Equal(0x17, announce.Flags);
    }

    [Fact]
    public void GeneratedTdfDecoder_DecodesVariableLengthArrays()
    {
        var packet = LoraRx.Decode(new byte[] { 0xFE, 0x34, 0x12, 0xAA, 0xBB, 0xCC });

        Assert.Equal(-2, packet.Snr);
        Assert.Equal(0x1234, packet.Rssi);
        Assert.Equal(new byte[] { 0xAA, 0xBB, 0xCC }, packet.Payload);
        Assert.IsNotType<byte[]>(packet.Payload);
    }

    [Fact]
    public void GeneratedTdfDecoder_ExposesNonByteArraysAsReadOnlyLists()
    {
        var stackFrame = ExceptionStackFrame.Decode(new byte[]
        {
            0x04, 0x03, 0x02, 0x01,
            0x08, 0x07, 0x06, 0x05,
        });

        Assert.Equal(new uint[] { 0x01020304u, 0x05060708u }, stackFrame.Frame);
        Assert.IsNotType<uint[]>(stackFrame.Frame);

        var cells = LteTacCells.Decode(new byte[]
        {
            0x01, 0x00,
            0x02, 0x00,
            0x06, 0x05, 0x04, 0x03,
            0x08, 0x07,
            0x0C, 0x0B, 0x0A, 0x09,
            0x10,
            0xF0,
            0x14, 0x13, 0x12, 0x11,
            0x16, 0x15,
            0x18, 0x17,
            0x19,
            0xEA,
        });

        Assert.Single(cells.Neighbours);
        Assert.IsNotType<TdfStructLteCellNeighbour[]>(cells.Neighbours);
    }

    [Fact]
    public void GeneratedDecoder_AppliesScalarConversions()
    {
        var reading = AmbientTempPresHum.Decode(new byte[]
        {
            0x88, 0x13, 0x00, 0x00,
            0x88, 0x13, 0x00, 0x00,
            0x34, 0x12,
        });

        Assert.Equal(5.0, reading.Temperature);
        Assert.Equal(5.0, reading.Pressure);
        Assert.Equal(46.6, reading.Humidity, 3);
    }

    [Fact]
    public void GeneratedDecoder_AppliesByteArrayIntegerConversions()
    {
        var littleEndianAddress = TdfStructBtAddrLe.Decode(new byte[] { 1, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06 });
        var bigEndianAddress = TdfStructEui48.Decode(new byte[] { 0x01, 0x02, 0x03, 0x04, 0x05, 0x06 });

        Assert.Equal(1, littleEndianAddress.Type);
        Assert.Equal(0x060504030201UL, littleEndianAddress.Val);
        Assert.Equal(0x010203040506UL, bigEndianAddress.Val);
    }

    [Fact]
    public void GeneratedTdfDecoder_RejectsTrailingBytes()
    {
        Assert.Throws<ArgumentException>(() => BatterySoc.Decode(new byte[] { 50, 0 }));
    }
}
