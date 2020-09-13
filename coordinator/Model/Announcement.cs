using System;
using System.Collections.Generic;
using Karenia.Rurikawa.Helpers;

#pragma warning disable CS8618
namespace Karenia.Rurikawa.Models {

    public class Announcement {
        public FlowSnake Id { get; set; }

        public string Title { get; set; }

        public string Body { get; set; }

        public string Sender { get; set; }

        public DateTimeOffset SendTime { get; set; }

        public List<string> Tags { get; set; } = new List<string>();

        public AnnouncementKind Kind { get; set; } = AnnouncementKind.Generic;
    }

    public enum AnnouncementKind {
        Generic = 0,
        Info = 1,
        Warn = 2,
        Error = 3,
        Success = 4
    }
}
