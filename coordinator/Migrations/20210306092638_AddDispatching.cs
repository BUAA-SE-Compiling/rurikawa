using System;
using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations
{
    public partial class AddDispatching : Migration
    {
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<DateTimeOffset>(
                name: "dispatch_time",
                table: "jobs",
                nullable: true);

            migrationBuilder.AddColumn<DateTimeOffset>(
                name: "finish_time",
                table: "jobs",
                nullable: true);

            migrationBuilder.AddColumn<string>(
                name: "judger",
                table: "jobs",
                nullable: true);

            migrationBuilder.CreateIndex(
                name: "ix_jobs_dispatch_time",
                table: "jobs",
                column: "dispatch_time");

            migrationBuilder.CreateIndex(
                name: "ix_jobs_finish_time",
                table: "jobs",
                column: "finish_time");

            migrationBuilder.CreateIndex(
                name: "ix_jobs_judger",
                table: "jobs",
                column: "judger");

            migrationBuilder.CreateIndex(
                name: "ix_jobs_stage",
                table: "jobs",
                column: "stage");
        }

        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropIndex(
                name: "ix_jobs_dispatch_time",
                table: "jobs");

            migrationBuilder.DropIndex(
                name: "ix_jobs_finish_time",
                table: "jobs");

            migrationBuilder.DropIndex(
                name: "ix_jobs_judger",
                table: "jobs");

            migrationBuilder.DropIndex(
                name: "ix_jobs_stage",
                table: "jobs");

            migrationBuilder.DropColumn(
                name: "dispatch_time",
                table: "jobs");

            migrationBuilder.DropColumn(
                name: "finish_time",
                table: "jobs");

            migrationBuilder.DropColumn(
                name: "judger",
                table: "jobs");
        }
    }
}
