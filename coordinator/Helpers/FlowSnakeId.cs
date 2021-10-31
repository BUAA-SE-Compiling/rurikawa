using System;
using System.Diagnostics.CodeAnalysis;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.ModelBinding;

namespace Karenia.Rurikawa.Helpers {
    /// <summary>
    /// FlowSnake is a time-sortable unique ID generator based on Twitter Snowflake.
    /// </summary>
    [ModelBinder(typeof(FlowSnakeBinder))]
    public struct FlowSnake : IEquatable<FlowSnake>, IComparable<FlowSnake>, IComparable<long> {
        const int TIMESTAMP_BITS = 34;
        const int WORKER_ID_BITS = 12;
        const int SEQUENCE_BITS = 18;

        public static readonly FlowSnake MaxValue = new(long.MaxValue);
        public static readonly FlowSnake MinValue = new(0);
        private static readonly char[] alphabet = "0123456789abcdefghjkmnpqrstvwxyz".ToCharArray();
        private static readonly byte[] charToBase32 = {
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
            255, 255, 255, 255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 16, 17, 255, 18, 19, 255, 20,
            21, 255, 22, 23, 24, 25, 26, 255, 27, 28, 29, 30, 31, 255, 255, 255, 255, 255, 255, 10, 11,
            12, 13, 14, 15, 16, 17, 255, 18, 19, 255, 20, 21, 255, 22, 23, 24, 25, 26, 255, 27, 28, 29,
            30, 31
        };

        private static readonly ThreadLocal<int> workerId = new(() =>
            // some kind of hash result of process and thread ids
            (Environment.ProcessId * 19260817) + Thread.CurrentThread.ManagedThreadId
        );
        private static readonly ThreadLocal<long> lastGeneration = new(() => 0);
        private static readonly ThreadLocal<int> sequenceNumber = new(() => 0);
        private static readonly Random prng = new();

        public FlowSnake(long num) => Num = num;

        public FlowSnake(long time, int worker, int seq)
            => Num = ((time) << (WORKER_ID_BITS + SEQUENCE_BITS))
                | (((long)worker & ((1 << WORKER_ID_BITS) - 1)) << SEQUENCE_BITS)
                | ((long)seq & ((1 << SEQUENCE_BITS) - 1));

        public long Num { get; }

        public static FlowSnake Generate() {
            var time = DateTimeOffset.Now.ToUnixTimeSeconds();

            int seq;
            if (time <= lastGeneration.Value) {
                // because this value is thread-local, we don't need to worry about
                // race conditions
                seq = sequenceNumber.Value;
                sequenceNumber.Value = seq + 1;
                if (seq >= (1 << SEQUENCE_BITS)) {
                    throw new OverflowException("Sequence number overflow!");
                }
            } else {
                seq = prng.Next((1 << SEQUENCE_BITS) - (1 << (SEQUENCE_BITS - 2)));
                sequenceNumber.Value = seq + 1;
            }
            lastGeneration.Value = time;

            var worker = workerId.Value;
            return new FlowSnake(time, worker, seq);
        }

        public FlowSnake(string? val, bool parseEmptyAsZero = false) {
            var length = val?.Length ?? 0;
            if (parseEmptyAsZero && length == 0) {
                Num = 0;
                return;
            } else if (length < 13) {
                throw new ArgumentException(
                    $"Expected string length: at least 13, got: {length}"
                );
            }
            long num = 0;
            int i = 0;
            int j = 0;
            while (j < 13 && i < length) {
                if (val![i] == '-') {
                    i++; continue;
                }
                byte ch = charToBase32[val[i]];
                if (ch == 255) {
                    throw new ArgumentException($"Unknown character when parsing FlowSnake");
                }
                num <<= 5;
                num |= ch;
                i++;
                j++;
            }
            Num = num;
        }

        public string ToString(bool dash) {
            var sb = new StringBuilder(14);
            int bit0 = (int)(Num >> 60) & 31;
            sb.Append(alphabet[bit0]);
            for (int i = 11; i >= 0; i--) {
                sb.Append(alphabet[(int)((Num >> (5 * i)) & 31)]);
            }
            if (dash) {
                sb.Insert(7, '-');
            }
            return sb.ToString();
        }

        public override string ToString() => ToString(true);

        public DateTimeOffset ExtractTime() => DateTimeOffset.FromUnixTimeSeconds(
            Num >> (SEQUENCE_BITS + WORKER_ID_BITS)
        );

        public static implicit operator long(FlowSnake i) => i.Num;

        #region Comparisons
        public override bool Equals(object? obj) => obj is FlowSnake snake && Equals(snake);

        public bool Equals(FlowSnake other) => Num == other.Num;

        public override int GetHashCode() => HashCode.Combine(Num);

        public int CompareTo([AllowNull] long other) => Num.CompareTo(other);

        public int CompareTo([AllowNull] FlowSnake other) => Num.CompareTo(other.Num);

        public static bool operator ==(FlowSnake left, FlowSnake right) =>
            left.Equals(right);

        public static bool operator !=(FlowSnake left, FlowSnake right) =>
            !(left == right);

        public static bool operator <(FlowSnake left, FlowSnake right) =>
            left.CompareTo(right) < 0;

        public static bool operator <=(FlowSnake left, FlowSnake right) =>
            left.CompareTo(right) <= 0;

        public static bool operator >(FlowSnake left, FlowSnake right) =>
            left.CompareTo(right) > 0;

        public static bool operator >=(FlowSnake left, FlowSnake right) =>
            left.CompareTo(right) >= 0;

        public static bool operator <(long left, FlowSnake right) =>
            left.CompareTo(right) < 0;

        public static bool operator <=(long left, FlowSnake right) =>
            left.CompareTo(right) <= 0;

        public static bool operator >(long left, FlowSnake right) =>
            left.CompareTo(right) > 0;

        public static bool operator >=(long left, FlowSnake right) =>
            left.CompareTo(right) >= 0;

        public static bool operator <(FlowSnake left, long right) =>
            left.CompareTo(right) < 0;

        public static bool operator <=(FlowSnake left, long right) =>
            left.CompareTo(right) <= 0;

        public static bool operator >(FlowSnake left, long right) =>
            left.CompareTo(right) > 0;

        public static bool operator >=(FlowSnake left, long right) =>
            left.CompareTo(right) >= 0;

        #endregion
    }

    public class FlowSnakeJsonConverter : JsonConverter<FlowSnake> {
        private readonly bool writeAsString;

        public FlowSnakeJsonConverter(bool writeAsString = true) => this.writeAsString = writeAsString;

        public override bool CanConvert(Type typeToConvert) => typeToConvert == typeof(FlowSnake);

        public override FlowSnake Read(
            ref Utf8JsonReader reader,
            Type typeToConvert,
            JsonSerializerOptions options
        ) => (reader.TokenType == JsonTokenType.Number) ? new(reader.GetInt64()) : new(reader.GetString(), true);

        public override void Write(
            Utf8JsonWriter writer,
            FlowSnake value,
            JsonSerializerOptions options
        ) {
            if (writeAsString) {
                writer.WriteStringValue(value.ToString());
            } else {
                writer.WriteNumberValue(value.Num);
            }
        }
    }

    public class FlowSnakeBinder : IModelBinder {
        public Task BindModelAsync(ModelBindingContext bindingContext) {
            if (bindingContext == null) {
                throw new ArgumentNullException(nameof(bindingContext));
            }
            var modelName = bindingContext.ModelName;
            var valueProviderResult = bindingContext.ValueProvider.GetValue(modelName);
            if (valueProviderResult == ValueProviderResult.None) {
                return Task.CompletedTask;
            }
            bindingContext.ModelState.SetModelValue(modelName, valueProviderResult);
            var val = valueProviderResult.FirstValue;
            try {
                var val_parsed = new FlowSnake(val);
                bindingContext.Result = ModelBindingResult.Success(val_parsed);
                return Task.CompletedTask;
            } catch (ArgumentException e) {
                bindingContext.ModelState.AddModelError(modelName, e.Message);
                return Task.CompletedTask;
            }
        }
    }
}
