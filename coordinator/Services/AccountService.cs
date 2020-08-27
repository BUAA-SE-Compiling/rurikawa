using System.Linq;
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
