import { Component, Input, OnInit } from '@angular/core';
import { Announcement } from 'src/models/server-types';

@Component({
  selector: 'app-announcement-item',
  templateUrl: './announcement-item.component.html',
  styleUrls: ['./announcement-item.component.styl'],
})
export class AnnouncementItemComponent implements OnInit {
  constructor() {}

  get bodyBrief() {
    let lf = this.item.body.search('\n\n');
    if (lf < 0) lf = this.item.body.length;
    return this.item.body.substr(0, lf);
  }

  @Input() item: Announcement;

  ngOnInit(): void {}
}
