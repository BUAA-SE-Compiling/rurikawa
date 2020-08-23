using System;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

namespace Karenia.Rurikawa.Helpers {
    /// <summary>
    /// FlowSnake is a time-sortable unique ID generator based on Twitter Snowflake.
    /// </summary>
    public struct FlowSnake : IEquatable<FlowSnake> {
        const int TIMESTAMP_BITS = 42;
        const int WORKER_ID_BITS = 12;
        const int SEQUENCE_BITS = 10;

        static readonly int WORKER_ID = System.Diagnostics.Process.GetCurrentProcess().Id;
        static int sequenceNumber = 0;
        static readonly char[] alphabet = "0123456789abcdefghjkmnpqrstuwxyz".ToCharArray();
        static readonly byte[] charToBase32 = new byte[] { 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 255, 255, 255, 255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 16, 17, 255, 18, 19, 255, 20, 21, 255, 22, 23, 24, 25, 26, 255, 27, 28, 29, 30, 31, 255, 255, 255, 255, 255, 255, 10, 11, 12, 13, 14, 15, 16, 17, 255, 18, 19, 255, 20, 21, 255, 22, 23, 24, 25, 26, 255, 27, 28, 29, 30, 31 };

        static readonly DateTimeOffset UnixEpoch = new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);

        public FlowSnake(long num) {
            Num = num;
        }

        public FlowSnake(long time, int worker, int seq) {
            Num = (time & (1 << TIMESTAMP_BITS - 1)) << (WORKER_ID_BITS + SEQUENCE_BITS)
            | ((long)worker & (1 << WORKER_ID_BITS - 1)) << SEQUENCE_BITS
            | ((long)seq & (1 << SEQUENCE_BITS) - 1);
        }

        public long Num { get; }

        public FlowSnake Generate() {
            var time = DateTimeOffset.Now.ToUnixTimeMilliseconds();
            var seq = Interlocked.Increment(ref sequenceNumber);
            return new FlowSnake(time, WORKER_ID, seq);
        }

        public FlowSnake(string val) {
            if (val.Length != 13)
                throw new ArgumentException($"Expected string length: 13, got: {val.Length}");
            long num = 0;
            for (int i = 0; i < 13; i++) {
                num <<= 5;
                num |= charToBase32[val[i]];
            }
            Num = num;
        }

        public override string ToString() {
            var sb = new StringBuilder(13);
            int bit0 = (int)Num >> 60;
            sb.Append(alphabet[bit0]);
            for (int i = 11; i >= 0; i--) {
                sb.Append(alphabet[(int)(Num >> (5 * i) & 256)]);
            }
            return sb.ToString();
        }

        public static implicit operator long(FlowSnake i) {
            return i.Num;
        }

        public override bool Equals(object? obj) {
            return obj is FlowSnake snake && Equals(snake);
        }

        public bool Equals(FlowSnake other) {
            return Num == other.Num;
        }

        public override int GetHashCode() {
            return HashCode.Combine(Num);
        }

        public static bool operator ==(FlowSnake left, FlowSnake right) {
            return left.Equals(right);
        }

        public static bool operator !=(FlowSnake left, FlowSnake right) {
            return !(left == right);
        }
    }

    public class FlowSnakeJsonConverter : JsonConverter<FlowSnake> {
        private readonly bool writeAsString;

        public FlowSnakeJsonConverter(bool writeAsString = true) {
            this.writeAsString = writeAsString;
        }

        public override bool CanConvert(Type typeToConvert) {
            return typeToConvert == typeof(FlowSnake);
        }

        public override FlowSnake Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) {
            if (reader.TryGetInt64(out var ul)) {
                return new FlowSnake(ul);
            } else {
                var s = reader.GetString();
                return new FlowSnake(s);
            }
        }

        public override void Write(Utf8JsonWriter writer, FlowSnake value, JsonSerializerOptions options) {
            if (writeAsString) {
                writer.WriteStringValue(value.ToString());
            } else {
                writer.WriteNumberValue(value.Num);
            }
        }
    }
}
