using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;

namespace Karenia.Rurikawa.Helpers {
    public class StreamAsyncAdaptor : Stream {
        public StreamAsyncAdaptor(Stream underlying) => Underlying = underlying;

        public override bool CanRead => Underlying.CanRead;

        public override bool CanSeek => Underlying.CanSeek;

        public override bool CanWrite => Underlying.CanWrite;

        public override long Length => Underlying.Length;

        public override long Position {
            get => Underlying.Position;
            set => Underlying.Position = value;
        }

        public Stream Underlying { get; }

        public override void Flush()
            => Task.Run(() => FlushAsync(CancellationToken.None)).GetAwaiter().GetResult();

        public override Task FlushAsync(CancellationToken cancellationToken) => Underlying.FlushAsync(cancellationToken);

        public override ValueTask<int> ReadAsync(Memory<byte> buffer, CancellationToken cancellationToken = default)
            => Underlying.ReadAsync(buffer, cancellationToken);

        public override Task<int> ReadAsync(byte[] buffer, int offset, int count, CancellationToken cancellationToken)
            => Underlying.ReadAsync(buffer, offset, count, cancellationToken);

        public override int Read(byte[] buffer, int offset, int count)
            => Task.Run(() => Underlying.ReadAsync(buffer, offset, count)).Result;

        public override long Seek(long offset, SeekOrigin origin) => Underlying.Seek(offset, origin);

        public override void SetLength(long value) => Underlying.SetLength(value);

        public override void Write(byte[] buffer, int offset, int count)
            => Task.Run(() => Underlying.WriteAsync(buffer, offset, count)).GetAwaiter().GetResult();

        public override ValueTask WriteAsync(ReadOnlyMemory<byte> buffer, CancellationToken cancellationToken = default)
            => Underlying.WriteAsync(buffer, cancellationToken);

        public override Task WriteAsync(byte[] buffer, int offset, int count, CancellationToken cancellationToken)
            => Underlying.WriteAsync(buffer, offset, count, cancellationToken);

        public override IAsyncResult BeginWrite(byte[] buffer, int offset, int count, AsyncCallback? callback, object? state)
            => Underlying.BeginWrite(buffer, offset, count, callback, state);
    }
}

