using System.Linq;
using System.Security.Claims;
using System.Threading.Tasks;
using IdentityServer4.Models;
using IdentityServer4.Services;
using IdentityServer4.Stores;
using IdentityServer4.Validation;
using Karenia.Rurikawa.Models;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class IdentityService :
        IResourceOwnerPasswordValidator {
        private readonly AccountService accountService;
        private readonly ILogger<IdentityService> logger;

        public IdentityService(
            AccountService accountService,
            ILogger<IdentityService> logger
        ) {
            this.accountService = accountService;
            this.logger = logger;
        }

        async Task IResourceOwnerPasswordValidator.ValidateAsync(ResourceOwnerPasswordValidationContext context) {
            var username = context.UserName;
            var password = context.Password;
            var result = await accountService.VerifyUser(username, password);
            if (result) {
                context.Result = new GrantValidationResult(
                    subject: username,
                    authenticationMethod: "custom",
                    claims: new Claim[] { });
            } else {
                context.Result = new GrantValidationResult(
                    TokenRequestErrors.InvalidGrant,
                    "Bad username or password");
            }
        }
    }

    // public class RefreshTokenStore : IRefreshTokenStore {
    //     public Task<RefreshToken> GetRefreshTokenAsync(string refreshTokenHandle) {
    //         throw new System.NotImplementedException();
    //     }

    //     public Task RemoveRefreshTokenAsync(string refreshTokenHandle) {
    //         throw new System.NotImplementedException();
    //     }

    //     public Task RemoveRefreshTokensAsync(string subjectId, string clientId) {
    //         throw new System.NotImplementedException();
    //     }

    //     public Task<string> StoreRefreshTokenAsync(RefreshToken refreshToken) {
    //         throw new System.NotImplementedException();
    //     }

    //     public Task UpdateRefreshTokenAsync(string handle, RefreshToken refreshToken) {
    //         throw new System.NotImplementedException();
    //     }
    // }
}
