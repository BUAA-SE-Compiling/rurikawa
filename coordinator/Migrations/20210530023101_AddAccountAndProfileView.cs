using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations {
    public partial class AddAccountAndProfileView : Migration {
        protected override void Up(MigrationBuilder migrationBuilder) {
            migrationBuilder.CreateIndex(
                name: "ix_profiles_student_id",
                table: "profiles",
                column: "student_id");

            migrationBuilder.Sql(@"
create view account_and_profile
as select
    a.username,
    a.kind,
    p.email,
    p.student_id
from 
    accounts as a
    inner join profiles as p
        on a.username = p.username
");
        }

        protected override void Down(MigrationBuilder migrationBuilder) {
            migrationBuilder.Sql("drop view account_and_profile");

            migrationBuilder.DropIndex(
                name: "ix_profiles_student_id",
                table: "profiles");


        }
    }
}
