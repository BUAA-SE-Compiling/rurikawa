import { Component, OnInit } from '@angular/core';
import { FLOWSNAKE_MAX } from 'src/models/flowsnake';
import { Announcement } from 'src/models/server-types';
import { ApiService } from 'src/services/api_service';

const FETCH_SIZE = 20;

@Component({
  selector: 'app-announcement-list-page',
  templateUrl: './announcement-list-page.component.html',
  styleUrls: ['./announcement-list-page.component.styl'],
})
export class AnnouncementListPageComponent implements OnInit {
  constructor(private api: ApiService) {}

  list: Announcement[] = [];
  end: boolean = false;

  startId() {
    if (this.list.length > 0) return this.list[this.list.length].id;
    else return FLOWSNAKE_MAX;
  }

  appendData() {
    if (this.end) return;
    this.api.announcement.query(this.startId(), FETCH_SIZE, false).subscribe({
      next: (data) => {
        this.list.push(...data);
        if (data.length < FETCH_SIZE) this.end = true;
      },
    });
  }

  ngOnInit(): void {
    this.appendData();
  }
}
