using System;
using System.Collections.Generic;
using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations
{
    public partial class addidentitytypes : Migration
    {
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropPrimaryKey(
                name: "pk_token_entry",
                table: "token_entry");

            migrationBuilder.DropIndex(
                name: "ix_token_entry_access_token",
                table: "token_entry");

            migrationBuilder.DropColumn(
                name: "access_token",
                table: "token_entry");

            migrationBuilder.AddColumn<string>(
                name: "token",
                table: "token_entry",
                nullable: false,
                defaultValue: "");

            migrationBuilder.AddColumn<bool>(
                name: "is_single_use",
                table: "token_entry",
                nullable: false,
                defaultValue: false);

            migrationBuilder.AddColumn<DateTimeOffset>(
                name: "issued_time",
                table: "token_entry",
                nullable: false,
                defaultValue: new DateTimeOffset(new DateTime(1, 1, 1, 0, 0, 0, 0, DateTimeKind.Unspecified), new TimeSpan(0, 0, 0, 0, 0)));

            migrationBuilder.AddColumn<DateTimeOffset>(
                name: "last_use_time",
                table: "token_entry",
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "related_token",
                table: "token_entry",
                nullable: true);

            migrationBuilder.AddColumn<List<string>>(
                name: "scope",
                table: "token_entry",
                nullable: false);

            migrationBuilder.AddPrimaryKey(
                name: "pk_token_entry",
                table: "token_entry",
                column: "token");

            migrationBuilder.CreateIndex(
                name: "ix_token_entry_token",
                table: "token_entry",
                column: "token",
                unique: true);
        }

        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropPrimaryKey(
                name: "pk_token_entry",
                table: "token_entry");

            migrationBuilder.DropIndex(
                name: "ix_token_entry_token",
                table: "token_entry");

            migrationBuilder.DropColumn(
                name: "token",
                table: "token_entry");

            migrationBuilder.DropColumn(
                name: "is_single_use",
                table: "token_entry");

            migrationBuilder.DropColumn(
                name: "issued_time",
                table: "token_entry");

            migrationBuilder.DropColumn(
                name: "last_use_time",
                table: "token_entry");

            migrationBuilder.DropColumn(
                name: "related_token",
                table: "token_entry");

            migrationBuilder.DropColumn(
                name: "scope",
                table: "token_entry");

            migrationBuilder.AddColumn<string>(
                name: "access_token",
                table: "token_entry",
                type: "text",
                nullable: false,
                defaultValue: "");

            migrationBuilder.AddPrimaryKey(
                name: "pk_token_entry",
                table: "token_entry",
                column: "access_token");

            migrationBuilder.CreateIndex(
                name: "ix_token_entry_access_token",
                table: "token_entry",
                column: "access_token",
                unique: true);
        }
    }
}
