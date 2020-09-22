

using System;
using System.Reactive.Disposables;
using System.Threading;

namespace Karenia.Rurikawa.Helpers {
    public class RefCountFusedObservable<T> : IObservable<T> {
        private readonly IObservable<T> inner;
        private readonly Action onRefcountZero;
        private long observerCount = 0;
        private bool started = false;

        public long ObserverCount { get => Interlocked.Read(ref observerCount); }

        public RefCountFusedObservable(IObservable<T> inner, Action onRefcountZero) {
            this.inner = inner;
            this.onRefcountZero = onRefcountZero;
        }

        public IDisposable Subscribe(IObserver<T> observer) {
            if (started && Interlocked.Read(ref observerCount) == 0) {
                observer.OnCompleted();
                return Disposable.Empty;
            }
            started = true;
            Interlocked.Increment(ref observerCount);
            var sub = inner.Subscribe(observer);
            return new Subscription(sub, OnChildUnsubscribe);
        }

        private void OnChildUnsubscribe() {
            var obs = Interlocked.Decrement(ref observerCount);
            if (obs == 0) onRefcountZero();
        }

        private class Subscription : IDisposable {
            private bool disposed = false;
            private readonly Action onDispose;

            public IDisposable Inner { get; }

            public Subscription(IDisposable inner, Action onDispose) {
                Inner = inner;
                this.onDispose = onDispose;
            }

            public void Dispose() {
                if (!disposed) {
                    disposed = true;
                    onDispose();
                    Inner.Dispose();
                }
            }
        }
    }
}
