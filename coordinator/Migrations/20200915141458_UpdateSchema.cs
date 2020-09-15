using System;
using System.Collections.Generic;
using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations
{
    public partial class UpdateSchema : Migration
    {
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<string>(
                name: "revision",
                table: "jobs",
                nullable: false,
                defaultValue: "");

            migrationBuilder.CreateTable(
                name: "announcements",
                columns: table => new
                {
                    id = table.Column<long>(nullable: false),
                    title = table.Column<string>(nullable: false),
                    body = table.Column<string>(nullable: false),
                    sender = table.Column<string>(nullable: false),
                    send_time = table.Column<DateTimeOffset>(nullable: false),
                    tags = table.Column<List<string>>(nullable: false),
                    kind = table.Column<int>(nullable: false)
                },
                constraints: table =>
                {
                    table.PrimaryKey("pk_announcements", x => x.id);
                });

            migrationBuilder.CreateIndex(
                name: "ix_announcements_id",
                table: "announcements",
                column: "id",
                unique: true);
        }

        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropTable(
                name: "announcements");

            migrationBuilder.DropColumn(
                name: "revision",
                table: "jobs");
        }
    }
}
