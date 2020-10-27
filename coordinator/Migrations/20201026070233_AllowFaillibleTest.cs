using Microsoft.EntityFrameworkCore.Migrations;

namespace Karenia.Rurikawa.Coordinator.Migrations {
    public partial class AllowFaillibleTest : Migration {
        protected override void Up(MigrationBuilder migrationBuilder) {
            migrationBuilder.Sql(@"
update test_suites
set test_groups = (
    select jsonb_object_agg(
        key, 
        value
    ) from (
        -- map $name => {name: $name, has_out: true, should_fail: false}
        select key, json_agg((select jsonb_build_object (
                'Name', val,
                'HasOut', true,
                'ShouldFail', false
            ))) as value
        from (
            select 
                key, 
                jsonb_array_elements(test_groups->key) as val 
            from jsonb_each(test_groups)
        ) as elements
        group by key
    ) as sub
) || '{}'::jsonb
            ");
        }

        protected override void Down(MigrationBuilder migrationBuilder) {
            migrationBuilder.Sql(@"
update test_suites
set test_groups = (
	select jsonb_object_agg(
		key, 
		value
	) from (
        -- map {name: $name, ..} => $name
		select key, json_agg(val->'Name') as value
		from (
			select 
				key, 
				jsonb_array_elements(test_groups->key) as val 
			from jsonb_each(test_groups)
		) as elements
		group by key
	) as sub
)
            ");
        }
    }
}
