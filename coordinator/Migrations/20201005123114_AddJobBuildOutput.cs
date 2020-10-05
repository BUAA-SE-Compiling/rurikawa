using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations
{
    public partial class AddJobBuildOutput : Migration
    {
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<string>(
                name: "build_output_file",
                table: "jobs",
                nullable: true);
        }

        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropColumn(
                name: "build_output_file",
                table: "jobs");
        }
    }
}
