using System;
using System.Buffers.Text;
using System.Collections.Generic;
using System.IdentityModel.Tokens.Jwt;
using System.Linq;
using System.Security.Claims;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using BCrypt.Net;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Auth;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Tokens;
using Npgsql;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class AccountService {
        private readonly RurikawaDb db;
        private readonly AuthInfo authInfo;
        private readonly JsonSerializerOptions jsonSerializerOptions;
        private readonly ILogger<AccountService> logger;

        public AccountService(
            RurikawaDb db,
            AuthInfo authInfo,
            JsonSerializerOptions jsonSerializerOptions,
            ILogger<AccountService> logger) {
            this.db = db;
            this.authInfo = authInfo;
            this.jsonSerializerOptions = jsonSerializerOptions;
            this.logger = logger;
        }

        public async Task CreateAccount(
            string username,
            string password,
            AccountKind kind = AccountKind.User) {
            var hashedPassword = HashPasswordWithGeneratedSalt(password);
            var account = new UserAccount
            {
                Username = username,
                HashedPassword = hashedPassword,
                Kind = kind
            };

            try {
                await db.Accounts.AddAsync(account);
                await db.SaveChangesAsync();
            } catch (DbUpdateException e) {
                if (e.InnerException is PostgresException ex) {
                    switch (ex.SqlState) {
                        case PostgresErrorCodes.UniqueViolation:
                        case PostgresErrorCodes.DuplicateObject:
                            throw new UsernameNotUniqueException(username, e);
                        default:
                            throw e;
                    }
                } else {
                    throw e;
                }
            }
            return;
        }

        public async ValueTask<bool> VerifyUser(
            string username,
            string password
        ) {
            var account = await db.Accounts.AsQueryable()
                .Where(a => a.Username == username)
                .SingleOrDefaultAsync();
            if (account == null) return false;
            return VerifyPassword(password, account.HashedPassword);
        }

        public async ValueTask<UserAccount> GetAccount(
            string username
        ) {
            var account = await db.Accounts.AsQueryable()
                .Where(a => a.Username == username)
                .SingleOrDefaultAsync();
            return account;
        }

        public enum EditPasswordResult { AccountNotFound, Success, Failure }

        public async ValueTask<EditPasswordResult> EditPassword(
            string username,
            string originalPassword,
            string newPassword) {
            var account = await db.Accounts.AsQueryable()
                .Where(a => a.Username == username)
                .SingleOrDefaultAsync();
            if (account == null) return EditPasswordResult.AccountNotFound;

            var verifyResult = VerifyPassword(originalPassword, account.HashedPassword);
            if (!verifyResult) return EditPasswordResult.Failure;

            var newHashedPassword = HashPasswordWithGeneratedSalt(newPassword);
            account.HashedPassword = newHashedPassword;
            await db.SaveChangesAsync();
            return EditPasswordResult.Success;
        }

        const int TOKEN_LENGTH = 32;
        static readonly char[] TOKEN_ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_+-.".ToCharArray();


        public string GenerateToken() {
            var sb = new StringBuilder(TOKEN_LENGTH);
            for (int i = 0; i < TOKEN_LENGTH; i++) {
                sb.Append(TOKEN_ALPHABET[System.Security.Cryptography.RandomNumberGenerator.GetInt32(TOKEN_ALPHABET.Length)]);
            }
            return sb.ToString();
        }

        public string CreateNewJwtAccessToken(UserAccount user, List<string> scope, DateTimeOffset expireTime) {
            var tokenHandler = new JwtSecurityTokenHandler();
            var key = authInfo.SigningKey;
            var tokenDescriptor = new SecurityTokenDescriptor()
            {
                Claims = new Dictionary<string, object>(){
                    {"sub", user.Username},
                    {"role", user.Kind.ToString()},
                    {"scope", scope}
            },
                IssuedAt = DateTime.UtcNow,
                Expires = expireTime.UtcDateTime,
                SigningCredentials = new SigningCredentials(key, SecurityAlgorithms.EcdsaSha256)
            };
            var token = tokenHandler.CreateToken(tokenDescriptor);
            return tokenHandler.WriteToken(token);
        }

        public string CreateNewShortLivingToken(string username, TimeSpan lifespan) {
            var crypto = authInfo.SigningKey.CryptoProviderFactory.CreateForSigning(authInfo.SigningKey, "ES256");

            var expiresBefore = DateTimeOffset.Now.Add(lifespan);

            var wsAuth = new WebsocketAuthInfo()
            {
                Username = username,
                ExpireBefore = expiresBefore
            };
            var json = JsonSerializer.SerializeToUtf8Bytes(wsAuth, jsonSerializerOptions);
            var signature = crypto.Sign(json);

            var resultBuilder = new StringBuilder();
            resultBuilder.Append(Convert.ToBase64String(json));
            resultBuilder.Append(".");
            resultBuilder.Append(Convert.ToBase64String(signature));
            var token = resultBuilder.ToString();
            token = token.Replace('+', '_');
            return token;
        }

        public async Task<string> CreateNewJudgerToken(
            DateTimeOffset? expireAt,
            bool isSingleUse,
            List<string> tags) {
            var token = GenerateToken();
            db.JudgerRegisterTokens.Add(
                new JudgerTokenEntry
                {
                    Token = token,
                    IssuedTime = DateTimeOffset.Now,
                    IsSingleUse = isSingleUse,
                    Expires = expireAt,
                    Tags = tags
                }
            );
            await db.SaveChangesAsync();
            return token;
        }

        public string? VerifyShortLivingToken(string token) {
            token = token.Replace('_', '+');
            var timestamp = DateTimeOffset.Now;
            var parts = token.Split('.');
            if (parts.Length != 2) return null;
            try {
                var info = Convert.FromBase64String(parts[0]);
                var signature = Convert.FromBase64String(parts[1]);
                var wsAuthInfo = JsonSerializer.Deserialize<WebsocketAuthInfo>(info, jsonSerializerOptions);
                var crypto = authInfo.SigningKey.CryptoProviderFactory.CreateForSigning(authInfo.SigningKey, "ES256");
                var result = crypto.Verify(info, signature);
                var expired = timestamp > wsAuthInfo.ExpireBefore;
                if (result && !expired) {
                    return wsAuthInfo.Username;
                } else {
                    return null;
                }
            } catch (Exception e) {
                logger.LogWarning(e, "Failed to verify token {0}", token);
                return null;
            }
        }

        public async Task<string> CreateNewAlternateAccessToken(
            string username,
            string? alternativeName,
            List<string> scope,
            DateTimeOffset? expireTime) {
            var accessToken = GenerateToken();
            db.AccessTokens.Add(new AccessTokenEntry
            {
                Username = username,
                Token = accessToken,
                IssuedTime = DateTimeOffset.Now,
                Scope = scope,
                TokenName = alternativeName,
                Expires = expireTime
            });
            await db.SaveChangesAsync();
            return accessToken;
        }

        public async Task<string> CreateNewRefreshToken(
            string username,
            string? relatedAccessToken,
            List<string> scope,
            DateTimeOffset? expireTime,
            bool isSingleUse) {
            var refreshToken = GenerateToken();
            db.RefreshTokens.Add(new RefreshTokenEntry
            {
                Username = username,
                Token = refreshToken,
                IssuedTime = DateTimeOffset.Now,
                Scope = scope,
                IsSingleUse = isSingleUse,
                RelatedToken = relatedAccessToken,
                Expires = expireTime
            });
            await db.SaveChangesAsync();
            return refreshToken;
        }

        /// <summary>
        /// Find the access token with token string as provided
        /// </summary>
        /// <param name="token"></param>
        /// <returns>Token, null if not found</returns>
        public async Task<TokenEntry?> GetAccessToken(string token) {
            return await GetToken(token, db.AccessTokens);
        }

        /// <summary>
        /// Find the refresh token with token string as provided
        /// </summary>
        /// <param name="token"></param>
        /// <returns>Token, null if not found</returns>
        public async Task<TokenEntry?> GetRefreshToken(string token) {
            return await GetToken(token, db.RefreshTokens);
        }

        /// <summary>
        /// Find the judger token with token string as provided
        /// </summary>
        /// <param name="token"></param>
        /// <returns>Token, null if not found</returns>
        public async Task<JudgerTokenEntry?> GetJudgerRegisterToken(string token) {
            return await GetToken(token, db.JudgerRegisterTokens);
        }

        /// <summary>
        /// Find the token with token string as provided
        /// </summary>
        /// <param name="token"></param>
        /// <param name="tokenSet"></param>
        /// <returns>Token, null if not found</returns>
        private async Task<T?> GetToken<T>(string token, DbSet<T> tokenSet) where T : TokenBase {
            var result = await tokenSet.Where(t => t.Token == token)
                .SingleOrDefaultAsync();
            if (result != null && result.IsExpired()) {
                tokenSet.Remove(result);
                await db.SaveChangesAsync();
                result = null;
            }
            if (result != null && result.IsSingleUse) {
                result.LastUseTime = DateTimeOffset.Now;
                await db.SaveChangesAsync();
            }
            return result;
        }

        public async Task<IList<AccessTokenEntry>> GetAllAccessToken(string username) {
            return await db.AccessTokens.Where(token => token.Username == username).ToListAsync();
        }

        public async Task<IList<RefreshTokenEntry>> GetAllRefreshToken(string username) {
            return await db.RefreshTokens.Where(token => token.Username == username).ToListAsync();
        }

        public string HashPasswordWithGeneratedSalt(string password) {
            return BCrypt.Net.BCrypt.EnhancedHashPassword(password, 11);
        }

        public bool VerifyPassword(string provided, string hashed) {
            return BCrypt.Net.BCrypt.EnhancedVerify(provided, hashed);
        }

        public async ValueTask<bool> IsInitialzed() {
            return await db.Accounts.AnyAsync(a => a.Kind == AccountKind.Root);
        }

        public async Task InitializeRootAccount(string username, string password) {
            if (await db.Accounts.AnyAsync(a => a.Kind == AccountKind.Root)) {
                throw new AlreadyInitializedException();
            }
            db.Accounts.Add(new UserAccount
            {
                Username = username,
                HashedPassword = HashPasswordWithGeneratedSalt(password),
                Kind = AccountKind.Root
            });
            await db.SaveChangesAsync();
        }

        public class AlreadyInitializedException : System.Exception { }

        public class UsernameNotUniqueException : System.Exception {
            public UsernameNotUniqueException(string username)
                : base($"Username {username}is not unique in database") {
                Username = username;
            }
            public UsernameNotUniqueException(string username, System.Exception inner)
                : base($"Username {username}is not unique in database", inner) {
                Username = username;
            }

            public string Username { get; }
        }
    }

    public class JudgerAuthenticateService : AuthenticationHandler<AuthenticationSchemeOptions> {
        public JudgerAuthenticateService(
            ILogger<JudgerAuthenticateService> logger,
            RurikawaDb db1,
            AccountService accountService,
            Microsoft.Extensions.Options.IOptionsMonitor<AuthenticationSchemeOptions> options,
            ILoggerFactory logger1,
            System.Text.Encodings.Web.UrlEncoder encoder,
            ISystemClock clock) : base(options, logger1, encoder, clock) {
            this.logger = logger;
            this.db1 = db1;
            this.accountService = accountService;
        }

        private readonly ILogger<JudgerAuthenticateService> logger;
        private readonly RurikawaDb db1;
        private readonly AccountService accountService;

        protected async Task<AuthenticateResult> _AuthenticateAsync() {
            KeyValuePair<string, Microsoft.Extensions.Primitives.StringValues> hdr;
            try {
                hdr = this.Request.Headers.Where(h => h.Key.ToLower() == "authorization").Single();
            } catch {
                return AuthenticateResult.Fail("No authorization header was found");
            }
            var token = await this.db1.Judgers
                .Where(judger => judger.Id == hdr.Value.First())
                .SingleOrDefaultAsync();

            if (token == null) {
                return AuthenticateResult.Fail("Unable to find token");
            }
            return AuthenticateResult.Success(new AuthenticationTicket(
                new ClaimsPrincipal(new ClaimsIdentity[]{
                    new ClaimsIdentity(new Claim[]{
                        new Claim(ClaimTypes.Role, "judger"),
                        new Claim(ClaimTypes.NameIdentifier, token.Id),
                    })
                }),
                new AuthenticationProperties(),
                this.Scheme.Name));
            throw new NotImplementedException();
        }

        protected override Task<AuthenticateResult> HandleAuthenticateAsync() {
            return _AuthenticateAsync();
        }
    }
}
