import { Component, OnInit, Input } from '@angular/core';
import { TestSuite } from 'src/models/server-types';
import { extractTime } from 'src/models/flowsnake';

@Component({
  selector: 'app-admin-dashboard-item',
  templateUrl: './dashboard-item.component.html',
  styleUrls: ['./dashboard-item.component.less'],
})
export class AdminDashboardItemComponent implements OnInit {
  constructor() {}

  @Input() item: TestSuite;

  get time() {
    return extractTime(this.item.id).local().format('YYYY-MM-DD HH:mm');
  }

  ngOnInit(): void {}
}
