import { Component, Input, OnInit } from '@angular/core';
import { Announcement } from 'src/models/server-types';

@Component({
  selector: 'app-announcement-item',
  templateUrl: './announcement-item.component.html',
  styleUrls: ['./announcement-item.component.styl'],
})
export class AnnouncementItemComponent implements OnInit {
  constructor() {}

  @Input() announcement: Announcement;

  ngOnInit(): void {}
}
