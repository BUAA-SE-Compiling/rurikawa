using System;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using BCrypt.Net;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Npgsql;

namespace Karenia.Rurikawa.Coordinator.Services {
    public class AccountService {
        private readonly RurikawaDb db;
        private readonly ILogger<AccountService> logger;

        public AccountService(RurikawaDb db, ILogger<AccountService> logger) {
            this.db = db;
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
            } catch (PostgresException e) {
                switch (e.SqlState) {
                    case PostgresErrorCodes.UniqueViolation:
                        throw new UsernameNotUniqueException(username, e);
                    default:
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
        readonly char[] TOKEN_ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_+-.".ToCharArray();
        public string GenerateToken() {
            var sb = new StringBuilder(TOKEN_LENGTH);
            for (int i = 0; i < TOKEN_LENGTH; i++) {
                sb.Append(TOKEN_ALPHABET[System.Security.Cryptography.RandomNumberGenerator.GetInt32(TOKEN_ALPHABET.Length)]);
            }
            return sb.ToString();
        }

        public async Task<string> CreateNewAccessToken(
            string username,
            string? alternativeName,
            DateTimeOffset? expireTime) {
            var accessToken = GenerateToken();
            db.AccessTokens.Add(new TokenEntry(username, accessToken, alternativeName, expireTime));
            await db.SaveChangesAsync();
            return accessToken;
        }

        public async Task<string> CreateNewRefreshToken(
            string username,
            DateTimeOffset? expireTime) {
            var refreshToken = GenerateToken();
            db.RefreshTokens.Add(new TokenEntry(username, refreshToken, null, expireTime));
            await db.SaveChangesAsync();
            return refreshToken;
        }

        /// <summary>
        /// Find the corresponding account of this access token
        /// </summary>
        /// <param name="token"></param>
        /// <returns>Account username, null if not found</returns>
        public async Task<string?> GetUserByAccessToken(string token) {
            var result = await db.AccessTokens.Where(t => t.AccessToken == token)
                .SingleOrDefaultAsync();
            return result?.Username;
        }

        /// <summary>
        /// Find the corresponding account of this refresh token
        /// </summary>
        /// <param name="token"></param>
        /// <returns>Account username, null if not found</returns>
        public async Task<string?> GetUserByRefreshToken(string token) {
            var result = await db.RefreshTokens.Where(t => t.AccessToken == token)
                .SingleOrDefaultAsync();
            return result?.Username;
        }

        public string HashPasswordWithGeneratedSalt(string password) {
            return BCrypt.Net.BCrypt.EnhancedHashPassword(password, 11);
        }

        public bool VerifyPassword(string provided, string hashed) {
            return BCrypt.Net.BCrypt.EnhancedVerify(provided, hashed);
        }

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
}
