using System;
using System.Threading;
using System.Threading.Tasks;

namespace Karenia.Rurikawa.Helpers {
    public static class DisposableSemaphore {
        public static async ValueTask<LockHandle> LockAsync(this SemaphoreSlim sem) {
            await sem.WaitAsync();
            return new LockHandle(sem);
        }

        public class LockHandle : IDisposable {
            private SemaphoreSlim sem;
            private bool isDisposed;
            public LockHandle(SemaphoreSlim sem) {
                this.sem = sem;
                this.isDisposed = false;
            }

            public void Dispose() {
                Dispose(true);
            }

            private void Dispose(bool disposing) {
                if (!this.isDisposed) {
                    this.isDisposed = true;
                    sem.Release();
                }
            }

            ~LockHandle() {
                Dispose(false);
            }
        }
    }
}
