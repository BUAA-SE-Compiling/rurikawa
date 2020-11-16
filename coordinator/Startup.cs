using System;
using System.Collections.Generic;
using System.Net.WebSockets;
using System.Reflection;
using System.Security.Cryptography.X509Certificates;
using System.Text.Encodings;
using System.Text.Json;
using System.Threading.Tasks;
using Dahomey.Json;
using Dahomey.Json.Serialization.Conventions;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Test.SerDe;
using Microsoft.AspNetCore.Authentication.JwtBearer;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Tokens;


namespace Karenia.Rurikawa.Coordinator {
    public class Startup {
        public Startup(IConfiguration configuration) {
            Configuration = configuration;
        }

        public IConfiguration Configuration { get; }

        // This method gets called by the runtime. Use this method to add services to the container.
        public void ConfigureServices(IServiceCollection services) {
            services.AddLogging();

            // TODO: add real certificate
            var certificate = new X509Certificate2("certs/dev.pfx");
            var certificateKey = new X509SecurityKey(certificate);
            var securityKey = new ECDsaSecurityKey(ECDsaCertificateExtensions.GetECDsaPrivateKey(certificate));

            services.AddAuthentication(opt => {
                opt.DefaultAuthenticateScheme = JwtBearerDefaults.AuthenticationScheme;
                opt.DefaultChallengeScheme = JwtBearerDefaults.AuthenticationScheme;
            }).AddJwtBearer(opt => {
                opt.RequireHttpsMetadata = false;
                opt.SaveToken = true;
                opt.TokenValidationParameters = new TokenValidationParameters
                {
                    ValidateIssuerSigningKey = true,
                    IssuerSigningKey = securityKey,
                    ValidateIssuer = false,
                    ValidateAudience = false,
                };
            }).AddScheme<Microsoft.AspNetCore.Authentication.AuthenticationSchemeOptions, JudgerAuthenticateMiddleware>("judger", null);

            services.AddAuthorization(opt => {
                opt.AddPolicy("user", policy => policy.RequireRole("User", "Admin", "Root"));
                opt.AddPolicy("admin", policy => policy.RequireRole("Admin", "Root"));
                opt.AddPolicy("root", policy => policy.RequireRole("Root"));
                opt.AddPolicy("judger", policy => policy.RequireRole("judger").AddAuthenticationSchemes("judger"));
            });

            services.AddSingleton<Models.Auth.AuthInfo>(_ => new Models.Auth.AuthInfo
            {
                SigningKey = securityKey
            });

            // Setup database stuff
            var pgsqlLinkParams = Configuration.GetValue<string>("pgsqlLink");
            var alwaysMigrate = Configuration.GetValue<bool>("alwaysMigrate");
            services.AddSingleton(_ => new DbOptions
            {
                AlwaysMigrate = alwaysMigrate
            });
            var testStorageParams = new SingleBucketFileStorageService.Params();
            Configuration.GetSection("testStorage").Bind(testStorageParams);
            services.AddDbContextPool<Models.RurikawaDb>(options => {
                options.UseNpgsql(pgsqlLinkParams);
            });

            // Setup redis
            var redisConnString = Configuration.GetValue<string>("redisLink");
            services.AddSingleton(_ => new RedisService(redisConnString));

            services.AddSingleton(
                svc => new SingleBucketFileStorageService(
                    testStorageParams,
                    svc.GetService<ILogger<SingleBucketFileStorageService>>())
            );
            services.AddSingleton<JudgerCoordinatorService>();
            services.AddSingleton<FrontendUpdateService>();
            services.AddScoped<AccountService>();
            services.AddScoped<JudgerService>();
            services.AddScoped<ProfileService>();
            services.AddScoped<DbService>();
            services.AddSingleton<JudgerAuthenticateService>();
            services.AddSingleton<DbVacuumingService>();
            services.AddSingleton<JsonSerializerOptions>(_ =>
                SetupJsonSerializerOptions(new JsonSerializerOptions())
            );
            services.AddSingleton<GenericCacheService>();
            services.AddSingleton<RurikawaCacheService>();
            services.AddSwaggerDocument();
            services.AddRouting(options => { options.LowercaseUrls = true; });
            services.AddControllers().AddJsonOptions(opt => SetupJsonSerializerOptions(opt.JsonSerializerOptions));
        }

        public JsonSerializerOptions SetupJsonSerializerOptions(JsonSerializerOptions opt) {
            opt.PropertyNamingPolicy = JsonNamingPolicy.CamelCase;
            opt.Converters.Add(new FlowSnakeJsonConverter());
            opt.Converters.Add(new System.Text.Json.Serialization.JsonStringEnumConverter());
            opt.Converters.Add(new TestCaseDefinitionConverter());
            opt.SetupExtensions();
            opt.IgnoreNullValues = true;

            var dis = opt.GetDiscriminatorConventionRegistry();
            dis.ClearConventions();
            dis.RegisterConvention(new DefaultDiscriminatorConvention<string>(opt, "_t"));
            dis.RegisterType<Models.Judger.ClientStatusMsg>();
            dis.RegisterType<Models.Judger.JobProgressMsg>();
            dis.RegisterType<Models.Judger.JobResultMsg>();
            dis.RegisterType<Models.Judger.PartialResultMsg>();
            dis.RegisterType<Models.Judger.AbortJobServerMsg>();
            dis.RegisterType<Models.Judger.NewJobServerMsg>();
            dis.RegisterType<Models.Judger.JobOutputMsg>();
            dis.RegisterType<Models.WebsocketApi.JobStatusUpdateMsg>();
            dis.RegisterType<Models.WebsocketApi.JudgerStatusUpdateMsg>();
            dis.RegisterType<Models.WebsocketApi.NewJobUpdateMsg>();
            dis.RegisterType<Models.WebsocketApi.SubscribeMsg>();
            dis.RegisterType<Models.WebsocketApi.TestOutputUpdateMsg>();
            dis.RegisterType<Models.WebsocketApi.SubscribeOutputMsg>();
            dis.DiscriminatorPolicy = DiscriminatorPolicy.Always;

            opt.IgnoreNullValues = true;
            opt.AllowTrailingCommas = true;
            opt.ReadCommentHandling = JsonCommentHandling.Skip;
            return opt;
        }

        // This method gets called by the runtime. Use this method to configure the HTTP request pipeline.
        public void Configure(IApplicationBuilder app, IWebHostEnvironment env, IServiceProvider svc) {
            var logger = svc.GetService<ILogger<Startup>>();
            logger.LogInformation(
                "Starting | {1}: Version {0}",
                Assembly.GetEntryAssembly()?.GetName().Version?.ToString(),
                Assembly.GetEntryAssembly()?.GetName().Name);

            if (env.IsDevelopment()) {
                app.UseDeveloperExceptionPage();
            }

            if (!env.IsDevelopment()) { app.UseHttpsRedirection(); }
            app.UseCors(opt => {
                opt.AllowAnyOrigin().AllowAnyHeader().AllowAnyMethod();
            });

            app.UseOpenApi();
            app.UseSwaggerUi3();

            app.UseRouting();

            // TODO: Add websocket options
            WebSocketOptions ws_opt = new WebSocketOptions();
            ws_opt.AllowedOrigins.Add("*");
            ws_opt.AllowedOrigins.Add("localhost");
            ws_opt.KeepAliveInterval = new System.TimeSpan(0, 0, 20);
            app.UseWebSockets(ws_opt);

            app.UseAuthentication();
            app.UseAuthorization();

            // Add websocket acceptor
            app.Use(async (ctx, next) => {
                logger.LogInformation("{0}，{1}", ctx.Request.Path, ctx.WebSockets.IsWebSocketRequest);
                if (ctx.Request.Path == "/api/v1/judger/ws") {
                    if (ctx.WebSockets.IsWebSocketRequest) {
                        var svc = app.ApplicationServices.GetService<JudgerCoordinatorService>();
                        await svc.TryUseConnection(ctx);
                    } else {
                        ctx.Response.StatusCode = 400;
                        await ctx.Response.Body.WriteAsync(System.Text.Encoding.UTF8.GetBytes("Expected websocket connection"));
                        await ctx.Response.CompleteAsync();
                    }
                } else {
                    await next();
                }
            });
            app.Use(async (ctx, next) => {
                logger.LogInformation("{0}，{1}", ctx.Request.Path, ctx.WebSockets.IsWebSocketRequest);
                if (ctx.Request.Path == "/api/v1/tests/ws") {
                    if (ctx.WebSockets.IsWebSocketRequest) {
                        var svc = app.ApplicationServices.GetService<FrontendUpdateService>();
                        await svc.TryUseConnection(ctx);
                    } else {
                        ctx.Response.StatusCode = 400;
                        await ctx.Response.Body.WriteAsync(System.Text.Encoding.UTF8.GetBytes("Expected websocket connection"));
                        await ctx.Response.CompleteAsync();
                    }
                } else {
                    await next();
                }
            });

            // migrate database if needed
            if (svc.GetService<DbOptions>().AlwaysMigrate) {
                svc.GetService<RurikawaDb>().Database.Migrate();
            }

            // pre-initialize long-running services
            var coordinator = svc.GetService<JudgerCoordinatorService>();
            // coordinator.RevertJobStatus().AsTask().Wait();
            var vacuumingService = svc.GetService<DbVacuumingService>();
            vacuumingService.StartVacuuming();
            var client = svc.GetService<SingleBucketFileStorageService>();
            client.Check().Wait();

            app.UseEndpoints(endpoints => {
                endpoints.MapControllers();
            });
        }
    }
}
