using System;
using System.Threading;
using System.Threading.Tasks;

namespace Karenia.Rurikawa.Helpers {
    public static class DisposableSemaphore {
        public static async ValueTask<LockHandle> LockAsync(this SemaphoreSlim sem) {
            await sem.WaitAsync();
            return new LockHandle(sem);
        }

        /// <summary>
        /// A lock handle for an asynchronously locked SemaphoreSlim.
        /// </summary>
        public class LockHandle : IDisposable {
            private readonly SemaphoreSlim sem;
            private bool isDisposed;
            public LockHandle(SemaphoreSlim sem) {
                this.sem = sem;
                this.isDisposed = false;
            }

            public void Dispose() {
                Dispose(true);
            }

            // For the whole Disposable pattern, see here:
            // https://docs.microsoft.com/en-us/dotnet/standard/garbage-collection/implementing-dispose
            private void Dispose(bool disposing) {
                if (isDisposed) {
                    return;
                }

                if (disposing) {
                    // Dispose managed state (managed objects).
                }

                // Free unmanaged resources (unmanaged objects) and override a finalizer below.
                // And set large fields to null.
                _ = sem.Release();
                isDisposed = true;
            }

            ~LockHandle() {
                Dispose(false);
            }
        }
    }
}
