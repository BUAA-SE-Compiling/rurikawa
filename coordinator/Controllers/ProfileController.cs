using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Karenia.Rurikawa.Models.Account;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/profile")]
    [Authorize]
    public class ProfileController : ControllerBase {
        private readonly ProfileService service;

        public ProfileController(ProfileService service) {
            this.service = service;
        }

        [HttpGet("{username}")]
        public async Task<ActionResult<Profile>> GetProfile([FromRoute] string username) {
            var res = await service.GetProfile(username);
            if (res != null) {
                return res;
            } else {
                return NotFound();
            }
        }

        [HttpPut("{username}")]
        public async Task<ActionResult> UpsertProfile(
            [FromRoute] string username,
            [FromBody] Profile profile) {
            var username_ = AuthHelper.ExtractUsername(HttpContext.User);
            if (username_ != username) {
                return Unauthorized(new ErrorResponse("not_owner"));
            }
            await service.UpsertProfile(username, profile);
            return NoContent();
        }

        [HttpPost("{username}/init")]
        public async Task<ActionResult> InitProfile(
            [FromRoute] string username) {
            var username_ = AuthHelper.ExtractUsername(HttpContext.User);
            if (username_ != username) {
                return Unauthorized(new ErrorResponse("not_owner"));
            }
            await service.InitializeProfileIfNotExists(username);
            return NoContent();
        }
    }
}
