using System;
using System.Collections.Generic;
using Karenia.Rurikawa.Models.Test;
using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations
{
    public partial class init : Migration
    {
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.CreateTable(
                name: "Accounts",
                columns: table => new
                {
                    Username = table.Column<string>(nullable: false),
                    HashedPassword = table.Column<byte[]>(nullable: false),
                    Salt = table.Column<byte[]>(nullable: false),
                    Kind = table.Column<int>(nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Accounts", x => x.Username);
                });

            migrationBuilder.CreateTable(
                name: "Jobs",
                columns: table => new
                {
                    Id = table.Column<long>(nullable: false),
                    Account = table.Column<string>(nullable: false),
                    Repo = table.Column<string>(nullable: false),
                    Branch = table.Column<string>(nullable: true),
                    TestSuite = table.Column<long>(nullable: false),
                    Tests = table.Column<List<string>>(nullable: false),
                    Stage = table.Column<int>(nullable: false),
                    Results = table.Column<Dictionary<string, TestResult>>(type: "jsonb", nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Jobs", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Judgers",
                columns: table => new
                {
                    Id = table.Column<string>(nullable: false),
                    AlternateName = table.Column<string>(nullable: true),
                    Tags = table.Column<List<string>>(nullable: true),
                    AcceptUntaggedJobs = table.Column<bool>(nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Judgers", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "Profiles",
                columns: table => new
                {
                    Username = table.Column<string>(nullable: false),
                    Email = table.Column<string>(nullable: true),
                    StudentId = table.Column<string>(nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_Profiles", x => x.Username);
                });

            migrationBuilder.CreateTable(
                name: "TestSuites",
                columns: table => new
                {
                    Id = table.Column<long>(nullable: false),
                    Name = table.Column<string>(nullable: false),
                    Description = table.Column<string>(nullable: false),
                    Tags = table.Column<List<string>>(nullable: true),
                    PackageFileId = table.Column<string>(nullable: false),
                    TimeLimit = table.Column<int>(nullable: true),
                    MemoryLimit = table.Column<int>(nullable: true),
                    TestGroups = table.Column<Dictionary<string, List<string>>>(type: "jsonb", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_TestSuites", x => x.Id);
                });

            migrationBuilder.CreateTable(
                name: "TokenEntry",
                columns: table => new
                {
                    AccessToken = table.Column<string>(nullable: false),
                    Username = table.Column<string>(nullable: false),
                    TokenName = table.Column<string>(nullable: true),
                    Expires = table.Column<DateTimeOffset>(nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("PK_TokenEntry", x => x.AccessToken);
                });

            migrationBuilder.CreateIndex(
                name: "IX_Accounts_Kind",
                table: "Accounts",
                column: "Kind");

            migrationBuilder.CreateIndex(
                name: "IX_Accounts_Username",
                table: "Accounts",
                column: "Username");

            migrationBuilder.CreateIndex(
                name: "IX_Jobs_Account",
                table: "Jobs",
                column: "Account");

            migrationBuilder.CreateIndex(
                name: "IX_Jobs_Id",
                table: "Jobs",
                column: "Id");

            migrationBuilder.CreateIndex(
                name: "IX_Jobs_TestSuite",
                table: "Jobs",
                column: "TestSuite");

            migrationBuilder.CreateIndex(
                name: "IX_Judgers_Id",
                table: "Judgers",
                column: "Id");

            migrationBuilder.CreateIndex(
                name: "IX_Judgers_Tags",
                table: "Judgers",
                column: "Tags");

            migrationBuilder.CreateIndex(
                name: "IX_Profiles_Email",
                table: "Profiles",
                column: "Email");

            migrationBuilder.CreateIndex(
                name: "IX_Profiles_Username",
                table: "Profiles",
                column: "Username");

            migrationBuilder.CreateIndex(
                name: "IX_TestSuites_Id",
                table: "TestSuites",
                column: "Id");

            migrationBuilder.CreateIndex(
                name: "IX_TestSuites_Name",
                table: "TestSuites",
                column: "Name");

            migrationBuilder.CreateIndex(
                name: "IX_TokenEntry_AccessToken",
                table: "TokenEntry",
                column: "AccessToken");

            migrationBuilder.CreateIndex(
                name: "IX_TokenEntry_Expires",
                table: "TokenEntry",
                column: "Expires");

            migrationBuilder.CreateIndex(
                name: "IX_TokenEntry_TokenName",
                table: "TokenEntry",
                column: "TokenName");

            migrationBuilder.CreateIndex(
                name: "IX_TokenEntry_Username",
                table: "TokenEntry",
                column: "Username");
        }

        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "Accounts");

            migrationBuilder.DropTable(
                name: "Jobs");

            migrationBuilder.DropTable(
                name: "Judgers");

            migrationBuilder.DropTable(
                name: "Profiles");

            migrationBuilder.DropTable(
                name: "TestSuites");

            migrationBuilder.DropTable(
                name: "TokenEntry");
        }
    }
}
