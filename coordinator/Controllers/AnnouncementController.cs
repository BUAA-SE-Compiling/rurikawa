using System;
using System.Collections.Generic;
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

        [HttpGet]
        public async Task<List<Announcement>> GetAnnouncements([FromQuery] FlowSnake startId, int count, bool ascending) {
            return await dbService.GetAnnouncements(startId, ascending, count);
        }

        [HttpGet("{id}")]
        public async Task<ActionResult<Announcement>> GetAnnouncement([FromRoute] FlowSnake id) {
            var res = await dbService.GetAnnouncement(id);
            return res ?? (ActionResult<Announcement>)NotFound();
        }

        [HttpPut("{id}")]
        [Authorize("admin")]
        public async Task<ActionResult> PutAnnouncement([FromRoute] FlowSnake id, [FromBody] Announcement announcement) {
            try {
                announcement.Id = id;
                await dbService.EditAnnouncement(announcement);
                return Ok();
            } catch (ArgumentOutOfRangeException) {
                return NotFound();
            }
        }

        [HttpDelete("{id}")]
        [Authorize("admin")]
        public async Task<ActionResult> DeleteAnnouncement([FromRoute] FlowSnake id) {
            try {
                await dbService.DeleteAnnouncement(id);
                return Ok();
            } catch (ArgumentOutOfRangeException) {
                return NotFound();
            }
        }

        [HttpPost()]
        [Authorize("admin")]
        public async Task<ActionResult> PostAnnouncement([FromBody] Announcement announcement) {
            var id = await dbService.CreateAnnouncement(announcement);
            return Ok(id.ToString());
        }
    }
}
