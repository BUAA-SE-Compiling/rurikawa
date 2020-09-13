using System.Linq;
using System.Security.Claims;

namespace Karenia.Rurikawa.Helpers {
    public static class AuthHelper {
        public static string? ExtractUsername(ClaimsPrincipal user) {
            var username = user.Claims
                .Where(c => c.Type == ClaimTypes.NameIdentifier)
                .SingleOrDefault()?.Value;
            return username;
        }
    }
}
