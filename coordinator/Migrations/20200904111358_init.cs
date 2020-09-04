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
                name: "access_tokens",
                columns: table => new
                {
                    token = table.Column<string>(nullable: false),
                    username = table.Column<string>(nullable: false),
                    token_name = table.Column<string>(nullable: true),
                    scope = table.Column<List<string>>(nullable: false),
                    related_token = table.Column<string>(nullable: true),
                    issued_time = table.Column<DateTimeOffset>(nullable: false),
                    expires = table.Column<DateTimeOffset>(nullable: true),
                    is_single_use = table.Column<bool>(nullable: false),
                    last_use_time = table.Column<DateTimeOffset>(nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_access_tokens", x => x.token);
                });

            migrationBuilder.CreateTable(
                name: "accounts",
                columns: table => new
                {
                    username = table.Column<string>(nullable: false),
                    hashed_password = table.Column<string>(nullable: false),
                    kind = table.Column<int>(nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_accounts", x => x.username);
                });

            migrationBuilder.CreateTable(
                name: "jobs",
                columns: table => new
                {
                    id = table.Column<long>(nullable: false),
                    account = table.Column<string>(nullable: false),
                    repo = table.Column<string>(nullable: false),
                    branch = table.Column<string>(nullable: true),
                    test_suite = table.Column<long>(nullable: false),
                    tests = table.Column<List<string>>(nullable: false),
                    stage = table.Column<int>(nullable: false),
                    result_kind = table.Column<int>(nullable: true),
                    result_message = table.Column<string>(nullable: true),
                    results = table.Column<Dictionary<string, TestResult>>(type: "jsonb", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_jobs", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "judger_register_tokens",
                columns: table => new
                {
                    token = table.Column<string>(nullable: false),
                    username = table.Column<string>(nullable: false),
                    token_name = table.Column<string>(nullable: true),
                    scope = table.Column<List<string>>(nullable: false),
                    related_token = table.Column<string>(nullable: true),
                    issued_time = table.Column<DateTimeOffset>(nullable: false),
                    expires = table.Column<DateTimeOffset>(nullable: true),
                    is_single_use = table.Column<bool>(nullable: false),
                    last_use_time = table.Column<DateTimeOffset>(nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_judger_register_tokens", x => x.token);
                });

            migrationBuilder.CreateTable(
                name: "judgers",
                columns: table => new
                {
                    id = table.Column<string>(nullable: false),
                    alternate_name = table.Column<string>(nullable: true),
                    tags = table.Column<List<string>>(nullable: true),
                    accept_untagged_jobs = table.Column<bool>(nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_judgers", x => x.id);
                });

            migrationBuilder.CreateTable(
                name: "profiles",
                columns: table => new
                {
                    username = table.Column<string>(nullable: false),
                    email = table.Column<string>(nullable: true),
                    student_id = table.Column<string>(nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_profiles", x => x.username);
                });

            migrationBuilder.CreateTable(
                name: "refresh_tokens",
                columns: table => new
                {
                    token = table.Column<string>(nullable: false),
                    username = table.Column<string>(nullable: false),
                    token_name = table.Column<string>(nullable: true),
                    scope = table.Column<List<string>>(nullable: false),
                    related_token = table.Column<string>(nullable: true),
                    issued_time = table.Column<DateTimeOffset>(nullable: false),
                    expires = table.Column<DateTimeOffset>(nullable: true),
                    is_single_use = table.Column<bool>(nullable: false),
                    last_use_time = table.Column<DateTimeOffset>(nullable: true)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_refresh_tokens", x => x.token);
                });

            migrationBuilder.CreateTable(
                name: "test_suites",
                columns: table => new
                {
                    id = table.Column<long>(nullable: false),
                    name = table.Column<string>(nullable: false),
                    title = table.Column<string>(nullable: false),
                    description = table.Column<string>(nullable: false),
                    tags = table.Column<List<string>>(nullable: true),
                    package_file_id = table.Column<string>(nullable: false),
                    time_limit = table.Column<int>(nullable: true),
                    memory_limit = table.Column<int>(nullable: true),
                    test_groups = table.Column<Dictionary<string, List<string>>>(type: "jsonb", nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_test_suites", x => x.id);
                });

            migrationBuilder.CreateIndex(
                name: "ix_access_tokens_expires",
                table: "access_tokens",
                column: "expires");

            migrationBuilder.CreateIndex(
                name: "ix_access_tokens_token",
                table: "access_tokens",
                column: "token",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_access_tokens_token_name",
                table: "access_tokens",
                column: "token_name");

            migrationBuilder.CreateIndex(
                name: "ix_access_tokens_username",
                table: "access_tokens",
                column: "username");

            migrationBuilder.CreateIndex(
                name: "ix_accounts_kind",
                table: "accounts",
                column: "kind");

            migrationBuilder.CreateIndex(
                name: "ix_accounts_username",
                table: "accounts",
                column: "username",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_jobs_account",
                table: "jobs",
                column: "account");

            migrationBuilder.CreateIndex(
                name: "ix_jobs_id",
                table: "jobs",
                column: "id",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_jobs_test_suite",
                table: "jobs",
                column: "test_suite");

            migrationBuilder.CreateIndex(
                name: "ix_judger_register_tokens_expires",
                table: "judger_register_tokens",
                column: "expires");

            migrationBuilder.CreateIndex(
                name: "ix_judger_register_tokens_token",
                table: "judger_register_tokens",
                column: "token",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_judger_register_tokens_token_name",
                table: "judger_register_tokens",
                column: "token_name");

            migrationBuilder.CreateIndex(
                name: "ix_judger_register_tokens_username",
                table: "judger_register_tokens",
                column: "username");

            migrationBuilder.CreateIndex(
                name: "ix_judgers_id",
                table: "judgers",
                column: "id",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_judgers_tags",
                table: "judgers",
                column: "tags");

            migrationBuilder.CreateIndex(
                name: "ix_profiles_email",
                table: "profiles",
                column: "email");

            migrationBuilder.CreateIndex(
                name: "ix_profiles_username",
                table: "profiles",
                column: "username",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_refresh_tokens_expires",
                table: "refresh_tokens",
                column: "expires");

            migrationBuilder.CreateIndex(
                name: "ix_refresh_tokens_token",
                table: "refresh_tokens",
                column: "token",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_refresh_tokens_token_name",
                table: "refresh_tokens",
                column: "token_name");

            migrationBuilder.CreateIndex(
                name: "ix_refresh_tokens_username",
                table: "refresh_tokens",
                column: "username");

            migrationBuilder.CreateIndex(
                name: "ix_test_suites_id",
                table: "test_suites",
                column: "id",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_test_suites_name",
                table: "test_suites",
                column: "name",
                unique: true);
        }

        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "access_tokens");

            migrationBuilder.DropTable(
                name: "accounts");

            migrationBuilder.DropTable(
                name: "jobs");

            migrationBuilder.DropTable(
                name: "judger_register_tokens");

            migrationBuilder.DropTable(
                name: "judgers");

            migrationBuilder.DropTable(
                name: "profiles");

            migrationBuilder.DropTable(
                name: "refresh_tokens");

            migrationBuilder.DropTable(
                name: "test_suites");
        }
    }
}
