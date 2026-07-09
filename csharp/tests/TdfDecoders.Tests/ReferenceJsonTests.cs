using System.Collections;
using System.Globalization;
using System.Reflection;
using System.Security.Cryptography;
using System.Text.Json;
using TdfDecoders;
using Xunit;
using Xunit.Sdk;

namespace TdfDecoders.Tests;

public sealed class ReferenceJsonTests
{
    private const int StorageBlockSize = 512;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
    };

    [Fact]
    public void OutputMatchesReferenceJsonForCommonTestData()
    {
        var testDataDirectory = FindRepoRoot().FullName + "/common/test_data";
        var includeExtensions = TdfIdMappings.GetName(1025) is not null;
        var fixturePaths = Directory
            .GetFiles(testDataDirectory, "*.bin")
            .Where(path => includeExtensions || !IsExtensionFixture(path))
            .OrderBy(path => path)
            .ToArray();

        Assert.NotEmpty(fixturePaths);

        foreach (var fixturePath in fixturePaths)
        {
            var referencePath = Path.ChangeExtension(fixturePath, ".reference.json");
            var expectedJson = File.ReadAllText(referencePath);
            var actualJson = GenerateReferenceJson(fixturePath);

            AssertReferenceJsonMatches(fixturePath, expectedJson, actualJson);
        }
    }

    private static bool IsExtensionFixture(string path)
    {
        return Path.GetFileName(path).StartsWith("extension_", StringComparison.Ordinal);
    }

    private static void AssertReferenceJsonMatches(string fixturePath, string expected, string actual)
    {
        if (expected == actual)
        {
            return;
        }

        var expectedLines = expected.Split('\n');
        var actualLines = actual.Split('\n');
        var lineCount = Math.Max(expectedLines.Length, actualLines.Length);

        for (var index = 0; index < lineCount; index++)
        {
            var hasExpectedLine = index < expectedLines.Length;
            var hasActualLine = index < actualLines.Length;

            if (hasExpectedLine && hasActualLine)
            {
                var expectedLine = expectedLines[index];
                var actualLine = actualLines[index];
                if (expectedLine == actualLine)
                {
                    continue;
                }

                throw new XunitException(
                    $"Reference JSON mismatch for {fixturePath}{Environment.NewLine}" +
                    $"First difference at line {index + 1}{Environment.NewLine}" +
                    $"Expected: {ToDisplayLine(expectedLine)}{Environment.NewLine}" +
                    $"Actual:   {ToDisplayLine(actualLine)}{Environment.NewLine}" +
                    $"Expected line count: {expectedLines.Length}{Environment.NewLine}" +
                    $"Actual line count: {actualLines.Length}");
            }

            if (hasExpectedLine)
            {
                throw new XunitException(
                    $"Reference JSON mismatch for {fixturePath}{Environment.NewLine}" +
                    $"Actual output ended before expected line {index + 1}{Environment.NewLine}" +
                    $"Expected: {ToDisplayLine(expectedLines[index])}{Environment.NewLine}" +
                    $"Expected line count: {expectedLines.Length}{Environment.NewLine}" +
                    $"Actual line count: {actualLines.Length}");
            }

            throw new XunitException(
                $"Reference JSON mismatch for {fixturePath}{Environment.NewLine}" +
                $"Actual output has an extra line at {index + 1}{Environment.NewLine}" +
                $"Actual: {ToDisplayLine(actualLines[index])}{Environment.NewLine}" +
                $"Expected line count: {expectedLines.Length}{Environment.NewLine}" +
                $"Actual line count: {actualLines.Length}");
        }

        throw new XunitException(
            $"Reference JSON mismatch for {fixturePath}{Environment.NewLine}" +
            "Content differs outside line text, likely trailing newline or encoding differences" +
            $"{Environment.NewLine}Expected bytes: {expected.Length}{Environment.NewLine}" +
            $"Actual bytes: {actual.Length}");
    }

    private static string ToDisplayLine(string line)
    {
        return line
            .Replace("\r", "\\r", StringComparison.Ordinal)
            .Replace("\t", "\\t", StringComparison.Ordinal);
    }

    private static string GenerateReferenceJson(string fixturePath)
    {
        var data = File.ReadAllBytes(fixturePath);
        var result = DecodeStorageFixture(data);
        var reference = new Dictionary<string, object?>
        {
            ["format"] = "tdf-decoder-reference-v1",
            ["fixture"] = Path.GetFileName(fixturePath),
            ["byteLength"] = data.Length,
            ["sha256"] = Convert.ToHexString(SHA256.HashData(data)).ToLowerInvariant(),
            ["success"] = result.Success,
            ["readings"] = result.Readings.Select(ToReferenceReading).ToArray(),
            ["error"] = result.Error is null ? null : ToReferenceError(result.Error),
        };

        return JsonSerializer.Serialize(reference, JsonOptions) + Environment.NewLine;
    }

    private static TdfBlockDecodeResult DecodeStorageFixture(byte[] data)
    {
        var readings = new List<TdfReading>();

        for (var offset = 0; offset < data.Length; offset += StorageBlockSize)
        {
            var length = Math.Min(StorageBlockSize, data.Length - offset);
            var chunk = new byte[length];
            Array.Copy(data, offset, chunk, 0, length);

            var result = BlockDecoder.DecodeBlock(TdfPayloadFromStorageFixture(chunk));
            readings.AddRange(result.Readings);

            if (result.Error is not null)
            {
                return new TdfBlockDecodeResult(readings, result.Error);
            }
        }

        return new TdfBlockDecodeResult(readings);
    }

    private static byte[] TdfPayloadFromStorageFixture(byte[] data)
    {
        var payload = new byte[data.Length - 2];
        Array.Copy(data, 2, payload, 0, payload.Length);
        return payload;
    }

    private static Dictionary<string, object?> ToReferenceReading(TdfReading reading)
    {
        var item = new Dictionary<string, object?>
        {
            ["id"] = reading.Id,
            ["name"] = reading.Name,
            ["rawDataHex"] = ToHex(reading.RawData),
            ["timestamp"] = reading.Timestamp is null ? null : ToReferenceTimestamp(reading.Timestamp.Value),
            ["period"] = reading.Period,
            ["index"] = reading.Index,
            ["decodedPayload"] = reading.DecodedPayload.Select(ToReferencePayload).ToArray(),
        };

        return item;
    }

    private static Dictionary<string, object?> ToReferenceTimestamp(TdfTimestamp timestamp)
    {
        var unixTime = timestamp.ToUnixTime();
        return new Dictionary<string, object?>
        {
            ["epochTime"] = timestamp.EpochTime,
            ["gpsSeconds"] = timestamp.GpsSeconds,
            ["fractionalTicks"] = timestamp.FractionalTicks,
            ["unixSeconds"] = unixTime.Seconds,
            ["unixNanoseconds"] = unixTime.Nanoseconds,
        };
    }

    private static Dictionary<string, object?> ToReferencePayload(object payload)
    {
        return new Dictionary<string, object?>
        {
            ["type"] = payload.GetType().Name,
            ["fields"] = ToReferenceObject(payload),
        };
    }

    private static Dictionary<string, object?> ToReferenceError(TdfDecodeError error)
    {
        if (error.Kind is TdfDecodeErrorKind.TruncatedHeader)
        {
            return new Dictionary<string, object?>
            {
                ["kind"] = "UnexpectedEof",
                ["offset"] = null,
                ["id"] = null,
                ["message"] = "Block ended before a complete TDF header could be read",
                ["exceptionType"] = null,
            };
        }

        return new Dictionary<string, object?>
        {
            ["kind"] = error.Kind.ToString(),
            ["offset"] = error.Offset,
            ["id"] = error.Id,
            ["message"] = error.Message,
            ["exceptionType"] = error.Exception?.GetType().FullName,
        };
    }

    private static Dictionary<string, object?> ToReferenceObject(object value)
    {
        var fields = new Dictionary<string, object?>();
        var properties = value
            .GetType()
            .GetProperties(BindingFlags.Instance | BindingFlags.Public)
            .Where(property => property.GetMethod is not null && property.GetIndexParameters().Length == 0)
            .OrderBy(property => property.MetadataToken);

        foreach (var property in properties)
        {
            fields[ToCamelCase(property.Name)] = ToReferenceValue(property.GetValue(value));
        }

        return fields;
    }

    private static object? ToReferenceValue(object? value)
    {
        if (value is null)
        {
            return null;
        }

        if (value is string or bool or byte or sbyte or short or ushort or int or uint or long or ulong or float or double)
        {
            return value;
        }

        if (value is char character)
        {
            return character.ToString();
        }

        if (value is IEnumerable enumerable)
        {
            return enumerable.Cast<object?>().Select(ToReferenceValue).ToArray();
        }

        return ToReferenceObject(value);
    }

    private static string ToHex(IEnumerable<byte> bytes)
    {
        return string.Concat(bytes.Select(value => value.ToString("x2", CultureInfo.InvariantCulture)));
    }

    private static string ToCamelCase(string value)
    {
        return string.IsNullOrEmpty(value)
            ? value
            : char.ToLowerInvariant(value[0]) + value[1..];
    }

    private static DirectoryInfo FindRepoRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null)
        {
            if (Directory.Exists(Path.Combine(directory.FullName, "common", "test_data"))
                && Directory.Exists(Path.Combine(directory.FullName, "csharp")))
            {
                return directory;
            }

            directory = directory.Parent;
        }

        throw new DirectoryNotFoundException("Could not find repository root from the test output directory.");
    }
}
