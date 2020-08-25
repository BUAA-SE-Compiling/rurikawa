using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.HttpsPolicy;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Karenia.Rurikawa.Coordinator {
    public class Startup {
        public Startup(IConfiguration configuration) {
            Configuration = configuration;
        }

        public IConfiguration Configuration { get; }

        // This method gets called by the runtime. Use this method to add services to the container.
        public void ConfigureServices(IServiceCollection services) {
            services.AddLogging();

            // TODO: Change to real identity stuff
            services.AddIdentityServer()
                .AddInMemoryClients(Config.GetClients())
                .AddInMemoryApiResources(Config.GetApiResources());

            var pgsqlLinkParams = Environment.GetEnvironmentVariable("rk_pgsql_link");

            services.AddDbContext<Models.RurikawaDb>(options => {
                options.UseNpgsql(
                //    pgsqlLinkParams
                "."
                );
            }
);
            services.AddSingleton<JudgerCoordinatorService>();
            services.AddRouting(options => { options.LowercaseUrls = true; });
            services.AddControllers();
        }

        // This method gets called by the runtime. Use this method to configure the HTTP request pipeline.
        public void Configure(IApplicationBuilder app, IWebHostEnvironment env) {
            if (env.IsDevelopment()) {
                app.UseDeveloperExceptionPage();
            }

            if (!env.IsDevelopment()) { app.UseHttpsRedirection(); }

            app.UseRouting();
            // TODO: Add websocket options
            app.UseWebSockets();
            app.UseAuthorization();

            // Add websocket acceptor
            app.Use(async (ctx, next) => {
                if (ctx.WebSockets.IsWebSocketRequest) {
                    var svc = app.ApplicationServices.GetService<JudgerCoordinatorService>();
                    await svc.TryUseConnection(ctx);
                }
                await next();
            });

            app.UseEndpoints(endpoints => {
                endpoints.MapControllers();
            });
        }
    }
}
