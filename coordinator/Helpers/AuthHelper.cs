using System.Linq;
using System.Security.Claims;

namespace Karenia.Rurikawa.Helpers {
    public static class AuthHelper {
        public static string? ExtractUsername(ClaimsPrincipal user)
            => user.Claims.SingleOrDefault(c => c.Type == ClaimTypes.NameIdentifier)?.Value;
    }
}
