﻿// <auto-generated />
using System;
using System.Collections.Generic;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Test;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Infrastructure;
using Microsoft.EntityFrameworkCore.Storage.ValueConversion;
using Npgsql.EntityFrameworkCore.PostgreSQL.Metadata;

namespace Karenia.Rurikawa.Coordinator.Migrations
{
    [DbContext(typeof(RurikawaDb))]
    partial class RurikawaDbModelSnapshot : ModelSnapshot
    {
        protected override void BuildModel(ModelBuilder modelBuilder)
        {
#pragma warning disable 612, 618
            modelBuilder
                .HasAnnotation("Npgsql:ValueGenerationStrategy", NpgsqlValueGenerationStrategy.IdentityByDefaultColumn)
                .HasAnnotation("ProductVersion", "3.1.7")
                .HasAnnotation("Relational:MaxIdentifierLength", 63);

            modelBuilder.Entity("Karenia.Rurikawa.Models.Account.AccessTokenEntry", b =>
                {
                    b.Property<string>("Token")
                        .HasColumnName("token")
                        .HasColumnType("text");

                    b.Property<DateTimeOffset?>("Expires")
                        .HasColumnName("expires")
                        .HasColumnType("timestamp with time zone");

                    b.Property<bool>("IsSingleUse")
                        .HasColumnName("is_single_use")
                        .HasColumnType("boolean");

                    b.Property<DateTimeOffset>("IssuedTime")
                        .HasColumnName("issued_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<DateTimeOffset?>("LastUseTime")
                        .HasColumnName("last_use_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<string>("RelatedToken")
                        .HasColumnName("related_token")
                        .HasColumnType("text");

                    b.Property<List<string>>("Scope")
                        .IsRequired()
                        .HasColumnName("scope")
                        .HasColumnType("text[]");

                    b.Property<string>("TokenName")
                        .HasColumnName("token_name")
                        .HasColumnType("text");

                    b.Property<string>("Username")
                        .IsRequired()
                        .HasColumnName("username")
                        .HasColumnType("text");

                    b.HasKey("Token")
                        .HasName("pk_access_tokens");

                    b.HasIndex("Expires")
                        .HasName("ix_access_tokens_expires");

                    b.HasIndex("Token")
                        .IsUnique()
                        .HasName("ix_access_tokens_token");

                    b.HasIndex("TokenName")
                        .HasName("ix_access_tokens_token_name");

                    b.HasIndex("Username")
                        .HasName("ix_access_tokens_username");

                    b.ToTable("access_tokens");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Account.JudgerTokenEntry", b =>
                {
                    b.Property<string>("Token")
                        .HasColumnName("token")
                        .HasColumnType("text");

                    b.Property<DateTimeOffset?>("Expires")
                        .HasColumnName("expires")
                        .HasColumnType("timestamp with time zone");

                    b.Property<bool>("IsSingleUse")
                        .HasColumnName("is_single_use")
                        .HasColumnType("boolean");

                    b.Property<DateTimeOffset>("IssuedTime")
                        .HasColumnName("issued_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<DateTimeOffset?>("LastUseTime")
                        .HasColumnName("last_use_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<List<string>>("Tags")
                        .IsRequired()
                        .HasColumnName("tags")
                        .HasColumnType("text[]");

                    b.HasKey("Token")
                        .HasName("pk_judger_register_tokens");

                    b.HasIndex("Expires")
                        .HasName("ix_judger_register_tokens_expires");

                    b.HasIndex("Token")
                        .IsUnique()
                        .HasName("ix_judger_register_tokens_token");

                    b.ToTable("judger_register_tokens");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Account.Profile", b =>
                {
                    b.Property<string>("Username")
                        .HasColumnName("username")
                        .HasColumnType("text");

                    b.Property<string>("Email")
                        .HasColumnName("email")
                        .HasColumnType("text");

                    b.Property<string>("StudentId")
                        .HasColumnName("student_id")
                        .HasColumnType("text");

                    b.HasKey("Username")
                        .HasName("pk_profiles");

                    b.HasIndex("Email")
                        .HasName("ix_profiles_email");

                    b.HasIndex("Username")
                        .IsUnique()
                        .HasName("ix_profiles_username");

                    b.ToTable("profiles");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Account.RefreshTokenEntry", b =>
                {
                    b.Property<string>("Token")
                        .HasColumnName("token")
                        .HasColumnType("text");

                    b.Property<DateTimeOffset?>("Expires")
                        .HasColumnName("expires")
                        .HasColumnType("timestamp with time zone");

                    b.Property<bool>("IsSingleUse")
                        .HasColumnName("is_single_use")
                        .HasColumnType("boolean");

                    b.Property<DateTimeOffset>("IssuedTime")
                        .HasColumnName("issued_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<DateTimeOffset?>("LastUseTime")
                        .HasColumnName("last_use_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<string>("RelatedToken")
                        .HasColumnName("related_token")
                        .HasColumnType("text");

                    b.Property<List<string>>("Scope")
                        .IsRequired()
                        .HasColumnName("scope")
                        .HasColumnType("text[]");

                    b.Property<string>("TokenName")
                        .HasColumnName("token_name")
                        .HasColumnType("text");

                    b.Property<string>("Username")
                        .IsRequired()
                        .HasColumnName("username")
                        .HasColumnType("text");

                    b.HasKey("Token")
                        .HasName("pk_refresh_tokens");

                    b.HasIndex("Expires")
                        .HasName("ix_refresh_tokens_expires");

                    b.HasIndex("Token")
                        .IsUnique()
                        .HasName("ix_refresh_tokens_token");

                    b.HasIndex("TokenName")
                        .HasName("ix_refresh_tokens_token_name");

                    b.HasIndex("Username")
                        .HasName("ix_refresh_tokens_username");

                    b.ToTable("refresh_tokens");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Account.UserAccount", b =>
                {
                    b.Property<string>("Username")
                        .HasColumnName("username")
                        .HasColumnType("text");

                    b.Property<string>("HashedPassword")
                        .IsRequired()
                        .HasColumnName("hashed_password")
                        .HasColumnType("text");

                    b.Property<int>("Kind")
                        .HasColumnName("kind")
                        .HasColumnType("integer");

                    b.HasKey("Username")
                        .HasName("pk_accounts");

                    b.HasIndex("Kind")
                        .HasName("ix_accounts_kind");

                    b.HasIndex("Username")
                        .IsUnique()
                        .HasName("ix_accounts_username");

                    b.ToTable("accounts");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Announcement", b =>
                {
                    b.Property<long>("Id")
                        .HasColumnName("id")
                        .HasColumnType("bigint");

                    b.Property<string>("Body")
                        .IsRequired()
                        .HasColumnName("body")
                        .HasColumnType("text");

                    b.Property<int>("Kind")
                        .HasColumnName("kind")
                        .HasColumnType("integer");

                    b.Property<DateTimeOffset>("SendTime")
                        .HasColumnName("send_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<string>("Sender")
                        .IsRequired()
                        .HasColumnName("sender")
                        .HasColumnType("text");

                    b.Property<List<string>>("Tags")
                        .IsRequired()
                        .HasColumnName("tags")
                        .HasColumnType("text[]");

                    b.Property<string>("Title")
                        .IsRequired()
                        .HasColumnName("title")
                        .HasColumnType("text");

                    b.HasKey("Id")
                        .HasName("pk_announcements");

                    b.HasIndex("Id")
                        .IsUnique()
                        .HasName("ix_announcements_id");

                    b.ToTable("announcements");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Judger.Job", b =>
                {
                    b.Property<long>("Id")
                        .HasColumnName("id")
                        .HasColumnType("bigint");

                    b.Property<string>("Account")
                        .IsRequired()
                        .HasColumnName("account")
                        .HasColumnType("text");

                    b.Property<string>("Branch")
                        .HasColumnName("branch")
                        .HasColumnType("text");

                    b.Property<string>("BuildOutputFile")
                        .HasColumnName("build_output_file")
                        .HasColumnType("text");

                    b.Property<string>("Repo")
                        .IsRequired()
                        .HasColumnName("repo")
                        .HasColumnType("text");

                    b.Property<int?>("ResultKind")
                        .HasColumnName("result_kind")
                        .HasColumnType("integer");

                    b.Property<string>("ResultMessage")
                        .HasColumnName("result_message")
                        .HasColumnType("text");

                    b.Property<Dictionary<string, TestResult>>("Results")
                        .IsRequired()
                        .HasColumnName("results")
                        .HasColumnType("jsonb");

                    b.Property<string>("Revision")
                        .IsRequired()
                        .HasColumnName("revision")
                        .HasColumnType("text");

                    b.Property<int>("Stage")
                        .HasColumnName("stage")
                        .HasColumnType("integer");

                    b.Property<long>("TestSuite")
                        .HasColumnName("test_suite")
                        .HasColumnType("bigint");

                    b.Property<List<string>>("Tests")
                        .IsRequired()
                        .HasColumnName("tests")
                        .HasColumnType("text[]");

                    b.HasKey("Id")
                        .HasName("pk_jobs");

                    b.HasIndex("Account")
                        .HasName("ix_jobs_account");

                    b.HasIndex("Id")
                        .IsUnique()
                        .HasName("ix_jobs_id");

                    b.HasIndex("TestSuite")
                        .HasName("ix_jobs_test_suite");

                    b.ToTable("jobs");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Judger.JudgerEntry", b =>
                {
                    b.Property<string>("Id")
                        .HasColumnName("id")
                        .HasColumnType("text");

                    b.Property<bool>("AcceptUntaggedJobs")
                        .HasColumnName("accept_untagged_jobs")
                        .HasColumnType("boolean");

                    b.Property<string>("AlternateName")
                        .HasColumnName("alternate_name")
                        .HasColumnType("text");

                    b.Property<List<string>>("Tags")
                        .HasColumnName("tags")
                        .HasColumnType("text[]");

                    b.HasKey("Id")
                        .HasName("pk_judgers");

                    b.HasIndex("AcceptUntaggedJobs")
                        .HasName("ix_judgers_accept_untagged_jobs");

                    b.HasIndex("Id")
                        .IsUnique()
                        .HasName("ix_judgers_id");

                    b.HasIndex("Tags")
                        .HasName("ix_judgers_tags");

                    b.ToTable("judgers");
                });

            modelBuilder.Entity("Karenia.Rurikawa.Models.Test.TestSuite", b =>
                {
                    b.Property<long>("Id")
                        .HasColumnName("id")
                        .HasColumnType("bigint");

                    b.Property<string>("Description")
                        .IsRequired()
                        .HasColumnName("description")
                        .HasColumnType("text");

                    b.Property<DateTimeOffset?>("EndTime")
                        .HasColumnName("end_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<bool>("IsPublic")
                        .HasColumnName("is_public")
                        .HasColumnType("boolean");

                    b.Property<int?>("MemoryLimit")
                        .HasColumnName("memory_limit")
                        .HasColumnType("integer");

                    b.Property<string>("Name")
                        .IsRequired()
                        .HasColumnName("name")
                        .HasColumnType("text");

                    b.Property<string>("PackageFileId")
                        .IsRequired()
                        .HasColumnName("package_file_id")
                        .HasColumnType("text");

                    b.Property<DateTimeOffset?>("StartTime")
                        .HasColumnName("start_time")
                        .HasColumnType("timestamp with time zone");

                    b.Property<List<string>>("Tags")
                        .HasColumnName("tags")
                        .HasColumnType("text[]");

                    b.Property<Dictionary<string, List<string>>>("TestGroups")
                        .IsRequired()
                        .HasColumnName("test_groups")
                        .HasColumnType("jsonb");

                    b.Property<int?>("TimeLimit")
                        .HasColumnName("time_limit")
                        .HasColumnType("integer");

                    b.Property<string>("Title")
                        .IsRequired()
                        .HasColumnName("title")
                        .HasColumnType("text");

                    b.HasKey("Id")
                        .HasName("pk_test_suites");

                    b.HasIndex("Id")
                        .IsUnique()
                        .HasName("ix_test_suites_id");

                    b.HasIndex("Name")
                        .HasName("ix_test_suites_name");

                    b.ToTable("test_suites");
                });
#pragma warning restore 612, 618
        }
    }
}
