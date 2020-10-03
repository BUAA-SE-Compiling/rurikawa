using System.Threading.Tasks;
using Karenia.Rurikawa.Coordinator.Services;
using Karenia.Rurikawa.Helpers;
using Karenia.Rurikawa.Models;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Karenia.Rurikawa.Coordinator.Controllers {
    [ApiController]
    [Route("api/v1/announcement")]
    public class AnnouncementController : ControllerBase {
        private readonly DbService dbService;

        public AnnouncementController(DbService dbService) {
            this.dbService = dbService;
        }

        [HttpGet("{id}")]
        public async Task<ActionResult<Announcement>> GetAnnouncement([FromRoute] FlowSnake id) {
            var res = await dbService.GetAnnouncement(id);
            if (res != null) return res;
            else return NotFound();
        }

        [HttpPost("{id}")]
        [Authorize("admin")]
        public async Task<ActionResult> PostAnnouncement([FromBody] Announcement announcement) {
            var id = await dbService.CreateAnnouncement(announcement);
            return Ok(id.ToString());
        }
    }
}
