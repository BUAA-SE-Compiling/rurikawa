import { Component, OnInit, Input } from '@angular/core';
import { DashboardItem, TestStatus } from 'src/models/job-items';
import {
  SliderItem,
  SliderItemKind,
} from 'src/components/base-components/slider-view/slider-view.component';
import { Moment } from 'moment';
import * as moment from 'moment';

function dashboardTypeToSlider(item: TestStatus): SliderItemKind {
  switch (item) {
    case 'ac':
      return 'accept';
    case 'wa':
      return 'error';
    case 'ce':
      return 'disable';
    case 're':
      return 'error';
    case 'oe':
      return 'disable';
    case 'tle':
      return 'warn';
  }
}

@Component({
  selector: 'app-dashboard-item-component',
  templateUrl: './dashboard-item-component.component.html',
  styleUrls: ['./dashboard-item-component.component.styl'],
})
export class DashboardItemComponentComponent implements OnInit {
  constructor() {}

  @Input() item: DashboardItem;

  public get slider(): SliderItem[] {
    return this.item.status.map<SliderItem>((st) => {
      return {
        kind: dashboardTypeToSlider(st.status),
        num: st.cnt,
      };
    });
  }

  public get timeString(): string {
    return moment(this.item.endTime).format('YYYY-MM-DD hh:mm');
  }

  ngOnInit(): void {}
}
