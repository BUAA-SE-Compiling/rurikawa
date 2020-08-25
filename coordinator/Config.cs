using System.Collections.Generic;
using IdentityServer4;
using IdentityServer4.Models;
using Microsoft.Extensions.Configuration;

namespace Karenia.Rurikawa {
    public class Config {
        public static IConfiguration Configuration { get; set; }

        public static IEnumerable<Client> GetClients() {
            return new List<Client>
            {
                new Client
                {
                    ClientId = "client",
                    AllowedGrantTypes = GrantTypes.ResourceOwnerPasswordAndClientCredentials,
                    ClientSecrets =
                    {
                        new Secret("client".Sha256())
                    },
                    AllowedScopes = new[] {IdentityServerConstants.LocalApi.ScopeName},
                    // AllowedCorsOrigins=new[]{"*"}
                    AccessTokenLifetime = 3600 * 24,
                    RefreshTokenUsage = TokenUsage.ReUse
                },
            };
        }

        public static IEnumerable<ApiResource> GetApiResources() {
            return new List<ApiResource>
            {
                new ApiResource(IdentityServerConstants.LocalApi.ScopeName, "identityapi",
                    new string[] {"Name"})
            };
        }
    }
}
