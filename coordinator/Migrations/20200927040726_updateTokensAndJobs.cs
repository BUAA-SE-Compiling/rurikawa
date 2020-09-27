using System.Collections.Generic;
using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations
{
    public partial class updateTokensAndJobs : Migration
    {
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropIndex(
                name: "ix_test_suites_name",
                table: "test_suites");

            migrationBuilder.DropIndex(
                name: "ix_judger_register_tokens_token_name",
                table: "judger_register_tokens");

            migrationBuilder.DropIndex(
                name: "ix_judger_register_tokens_username",
                table: "judger_register_tokens");

            migrationBuilder.DropColumn(
                name: "related_token",
                table: "judger_register_tokens");

            migrationBuilder.DropColumn(
                name: "scope",
                table: "judger_register_tokens");

            migrationBuilder.DropColumn(
                name: "token_name",
                table: "judger_register_tokens");

            migrationBuilder.DropColumn(
                name: "username",
                table: "judger_register_tokens");

            migrationBuilder.AddColumn<List<string>>(
                name: "tags",
                table: "judger_register_tokens",
                nullable: false);

            migrationBuilder.CreateIndex(
                name: "ix_test_suites_name",
                table: "test_suites",
                column: "name");

            migrationBuilder.CreateIndex(
                name: "ix_judgers_accept_untagged_jobs",
                table: "judgers",
                column: "accept_untagged_jobs");
        }

        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropIndex(
                name: "ix_test_suites_name",
                table: "test_suites");

            migrationBuilder.DropIndex(
                name: "ix_judgers_accept_untagged_jobs",
                table: "judgers");

            migrationBuilder.DropColumn(
                name: "tags",
                table: "judger_register_tokens");

            migrationBuilder.AddColumn<string>(
                name: "related_token",
                table: "judger_register_tokens",
                type: "text",
                nullable: true);

            migrationBuilder.AddColumn<List<string>>(
                name: "scope",
                table: "judger_register_tokens",
                type: "text[]",
                nullable: false);

            migrationBuilder.AddColumn<string>(
                name: "token_name",
                table: "judger_register_tokens",
                type: "text",
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "username",
                table: "judger_register_tokens",
                type: "text",
                nullable: false,
                defaultValue: "");

            migrationBuilder.CreateIndex(
                name: "ix_test_suites_name",
                table: "test_suites",
                column: "name",
                unique: true);

            migrationBuilder.CreateIndex(
                name: "ix_judger_register_tokens_token_name",
                table: "judger_register_tokens",
                column: "token_name");

            migrationBuilder.CreateIndex(
                name: "ix_judger_register_tokens_username",
                table: "judger_register_tokens",
                column: "username");
        }
    }
}
