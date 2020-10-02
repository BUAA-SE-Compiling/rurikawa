using System.Reflection;
using Microsoft.AspNetCore.Mvc;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/status")]
    public class StatusController : ControllerBase {
        /// <summary>
        /// Always return 204.
        /// </summary>
        [HttpGet("ping")]
        public ActionResult Pong() { return NoContent(); }

        /// <summary>
        /// Get the name and version of the running assembly.
        /// </summary>
        /// <returns>
        ///     A string formatted in the following fashion:
        /// <code>
        ///     {AssemblyName}, Version={Version}, {AdditionalData}
        /// </code>
        /// </returns>
        [HttpGet("assembly")]
        public string? GetAssembly() {
            return Assembly.GetEntryAssembly()?.GetName().FullName;
        }
    }
}
