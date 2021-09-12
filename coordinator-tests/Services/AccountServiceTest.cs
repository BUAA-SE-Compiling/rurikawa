using System.Security.Cryptography;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Models;
using Microsoft.AspNetCore.Authentication;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Tokens;
using NUnit.Framework;

namespace Karenia.Rurikawa.Coordinator.Tests {
    [TestFixture]
    public class AccountServiceTests {
        private RurikawaDb db;
        private AccountService accountService;

        [Test]
        public async Task TestAccount() {
            await accountService.CreateAccount("root", "1234", Models.Account.AccountKind.Root);

            Assert.That(
                await accountService.VerifyUser("root", "1234"),
                "User can log in with the right password");
            Assert.That(
                !await accountService.VerifyUser("root", "12345"),
                "User cannot log in with the wrong password");

            await accountService.EditPassword("root", "1234", "12345");

            Assert.That(
                await accountService.VerifyUser("root", "12345"),
                "User can log in with the new password");
            Assert.That(
                !await accountService.VerifyUser("root", "1234"),
                "User cannot log in with the old password");
        }

        [SetUp]
        public void SetUp() {
            var key = new ECDsaSecurityKey(ECDsa.Create());
            db = Mock.CreateMockDatabase();
            accountService = new AccountService(db, new Models.Auth.AuthInfo { SigningKey = key }, new System.Text.Json.JsonSerializerOptions(), null);
        }

        [TearDown]
        public void TearDown() {
            db = null;
        }
    }
}
