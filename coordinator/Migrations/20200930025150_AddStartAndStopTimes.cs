using System;
using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations {
    public partial class AddStartAndStopTimes : Migration {
        protected override void Up(MigrationBuilder migrationBuilder) {
            migrationBuilder.AddColumn<DateTimeOffset>(
                name: "end_time",
                table: "test_suites",
                nullable: true);

            migrationBuilder.AddColumn<bool>(
                name: "is_public",
                table: "test_suites",
                nullable: false,
                defaultValue: true);

            migrationBuilder.AddColumn<DateTimeOffset>(
                name: "start_time",
                table: "test_suites",
                nullable: true);
        }

        protected override void Down(MigrationBuilder migrationBuilder) {
            migrationBuilder.DropColumn(
                name: "end_time",
                table: "test_suites");

            migrationBuilder.DropColumn(
                name: "is_public",
                table: "test_suites");

            migrationBuilder.DropColumn(
                name: "start_time",
                table: "test_suites");
        }
    }
}
