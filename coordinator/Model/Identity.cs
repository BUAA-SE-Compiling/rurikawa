using System.Collections.Generic;
using System.Text.Json.Serialization;
using Microsoft.IdentityModel.Tokens;

namespace Karenia.Rurikawa.Models.Auth {
    public class OAuth2Request {
        public string GrantType { get; set; }
        public string Scope { get; set; }
        public string ClientId { get; set; }
        public string ClientSecret { get; set; }
        [JsonExtensionData]
        public Dictionary<string, string> ExtraInfo { get; set; }
    }

    public class OAuth2Response {
        public string AccessToken { get; set; }
        public string TokenType { get; set; } = "bearer";
        public long? ExpiresIn { get; set; }
        public string? RefreshToken { get; set; }
        public string? Scope { get; set; }
    }

    public class AuthInfo {
        public SecurityKey SigningKey { get; set; }
    }

    public static class AuthConstants {
        public static readonly string WebhookScope = "";
    }
}
