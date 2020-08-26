using System.IO;
using System.Threading.Tasks;

namespace Karenia.Rurikawa.Helpers {
    public static class StreamExt {
        public static async Task DrainAsync(Stream s, int bufferLength = 4096) {
            var buf = new byte[bufferLength];
            int n = await s.ReadAsync(buf);
            while (n > 0)
                n = await s.ReadAsync(buf);
        }
    }
}
