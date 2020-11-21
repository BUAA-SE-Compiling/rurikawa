using System;
using System.Buffers.Text;
using System.Collections.Generic;
using System.IdentityModel.Tokens.Jwt;
using System.IO;
using System.Linq;
using System.Security.Claims;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using AsyncPrimitives;
using BCrypt.Net;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Karenia.Rurikawa.Models.Auth;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Tokens;
using Npgsql;
using StackExchange.Redis;

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

        private static readonly Regex usernameRestriction = new Regex("^[-_0-9a-zA-Z]{1,64}$");

        public async Task CreateAccount(
            string username,
            string password,
            AccountKind kind = AccountKind.User) {
            if (!usernameRestriction.IsMatch(username))
                throw new InvalidUsernameException(username);

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
        static readonly char[] TOKEN_ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-.=".ToCharArray();


        public static string GenerateToken() {
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
                : base($"Username {username} is not unique in database") {
                Username = username;
            }
            public UsernameNotUniqueException(string username, System.Exception inner)
                : base($"Username {username} is not unique in database", inner) {
                Username = username;
            }

            public string Username { get; }
        }

        public class InvalidUsernameException : System.Exception {
            public InvalidUsernameException(string username)
                : base($"Username {username} is invalid") {
                Username = username;
            }
            public InvalidUsernameException(string username, System.Exception inner)
                : base($"Username {username} is invalid", inner) {
                Username = username;
            }

            public string Username { get; }
        }
    }

    public class JudgerAuthenticateService {
        private readonly RedisService redis;
        private readonly IServiceProvider serviceProvider;
        private static readonly TimeSpan redisKeyLifetime = TimeSpan.FromHours(2);

        public JudgerAuthenticateService(
            RedisService redis,
            IServiceProvider serviceProvider
        ) {
            this.redis = redis;
            this.serviceProvider = serviceProvider;
        }

        public async ValueTask<bool> AuthenticateAsync(string token) {
            string key = $"auth:judger:{token}";

            var redisDb = await redis.GetDatabase();
            var res = await redisDb.StringGetAsync(key);

            if (res.IsNullOrEmpty) {
                var db = serviceProvider.GetService<RurikawaDb>();
                var valueExists = await db.Judgers.Where(judger => judger.Id == token)
                    .AnyAsync();

                if (valueExists) {
                    await redisDb.StringSetAsync(
                        key,
                        "1",
                        expiry: redisKeyLifetime,
                        flags: CommandFlags.FireAndForget);
                } else {
                    await redisDb.StringSetAsync(
                        key,
                        "0",
                        expiry: redisKeyLifetime,
                        flags: CommandFlags.FireAndForget);
                }

                return valueExists;
            } else {
                if (res == "0") return false;
                else return true;
            }
        }
    }

    public class JudgerAuthenticateMiddleware : AuthenticationHandler<AuthenticationSchemeOptions> {
        public JudgerAuthenticateMiddleware(
            JudgerAuthenticateService service,
            Microsoft.Extensions.Options.IOptionsMonitor<AuthenticationSchemeOptions> options,
            ILoggerFactory logger1,
            System.Text.Encodings.Web.UrlEncoder encoder,
            ISystemClock clock) : base(options, logger1, encoder, clock) {
            this.service = service;
        }

        private readonly JudgerAuthenticateService service;

        protected new async Task<AuthenticateResult> AuthenticateAsync() {
            var endpoint = Context.GetEndpoint();
            if (endpoint?.Metadata?.GetMetadata<IAllowAnonymous>() != null) {
                return AuthenticateResult.NoResult();
            }

            if (!this.Request.Headers.ContainsKey("Authorization")) {
                return AuthenticateResult.Fail("No authorization header was found");
            }
            var hdr = Request.Headers["Authorization"];
            string auth = hdr.First();


            if (!await this.service.AuthenticateAsync(auth)) {
                Logger.LogInformation("Auth failed with header {0}", auth);
                return AuthenticateResult.Fail("Unable to find token");
            }

            return AuthenticateResult.Success(new AuthenticationTicket(
                new ClaimsPrincipal(new ClaimsIdentity[]{
                    new ClaimsIdentity(new Claim[]{
                        new Claim(ClaimTypes.Role, "judger"),
                        new Claim(ClaimTypes.NameIdentifier, auth),
                    })
                }),
                new AuthenticationProperties(),
                this.Scheme.Name));
        }

        protected override Task<AuthenticateResult> HandleAuthenticateAsync() {
            return AuthenticateAsync();
        }

        protected override Task HandleChallengeAsync(AuthenticationProperties properties) {
            this.Response.StatusCode = 401;
            return base.HandleChallengeAsync(properties);
        }
    }

    public class TemporaryTokenAuthService {
        private readonly RedisService redis;
        private readonly ILogger<TemporaryTokenAuthService> logger;
        private readonly IServiceProvider serviceProvider;
        private static readonly TimeSpan redisKeyLifetime = TimeSpan.FromHours(2);

        public TemporaryTokenAuthService(
            RedisService redis,
            ILogger<TemporaryTokenAuthService> logger,
            IServiceProvider serviceProvider
        ) {
            this.redis = redis;
            this.logger = logger;
            this.serviceProvider = serviceProvider;
        }

        private static string GetKey(string token) => $"auth:temp:{token}";

        public async Task AddToken(string token, ClaimsIdentity identity, TimeSpan expiry) {
            string key = GetKey(token);
            var redisDb = await redis.GetDatabase();
            var stream = new MemoryStream();
            var writer = new BinaryWriter(stream);
            identity.WriteTo(writer);
            stream.Seek(0, SeekOrigin.Begin);
            byte[] value = stream.ReadToEnd();
            await redisDb.StringSetAsync(key, value, expiry: expiry);
        }

        public async ValueTask<ClaimsIdentity?> AuthenticateAsync(string token) {
            string key = GetKey(token);

            var redisDb = await redis.GetDatabase();
            var res = await redisDb.StringGetAsync(key);

            if (res.IsNullOrEmpty) {
                return null;
            } else {
                var resAsBytes = (byte[])res;
                var reader = new BinaryReader(new MemoryStream(resAsBytes));
                var identity = new ClaimsIdentity(reader);
                return identity;
            }
        }
    }

    public class TemporaryTokenAuthMiddleware : AuthenticationHandler<AuthenticationSchemeOptions> {
        public TemporaryTokenAuthMiddleware(
            TemporaryTokenAuthService service,
            Microsoft.Extensions.Options.IOptionsMonitor<AuthenticationSchemeOptions> options,
            ILoggerFactory logger1,
            System.Text.Encodings.Web.UrlEncoder encoder,
            ISystemClock clock) : base(options, logger1, encoder, clock) {
            this.service = service;
        }

        private readonly TemporaryTokenAuthService service;

        protected new async Task<AuthenticateResult> AuthenticateAsync() {
            var endpoint = Context.GetEndpoint();
            if (endpoint?.Metadata?.GetMetadata<IAllowAnonymous>() != null) {
                return AuthenticateResult.NoResult();
            }

            if (!this.Request.Query.ContainsKey("auth")) {
                return AuthenticateResult.Fail("No auth query header can be found");
            }
            var query = Request.Query["auth"];
            string auth = query.First();
            Logger.LogInformation($"auth token: {auth}");

            var res = await this.service.AuthenticateAsync(auth);
            if (res == null) {
                return AuthenticateResult.Fail("No such token was found");
            } else {
                return AuthenticateResult.Success(new AuthenticationTicket(
                    new ClaimsPrincipal(res),
                    new AuthenticationProperties(),
                    this.Scheme.Name));
            }
        }

        protected override Task<AuthenticateResult> HandleAuthenticateAsync() {
            return AuthenticateAsync();
        }

        protected override Task HandleChallengeAsync(AuthenticationProperties properties) {
            this.Response.StatusCode = 401;
            return base.HandleChallengeAsync(properties);
        }
    }
}
