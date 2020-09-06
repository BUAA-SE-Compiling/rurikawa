import { Component, OnInit, Input } from '@angular/core';
import {
  DashboardItem,
  TestStatus,
  dashboardTypeToSlider,
} from 'src/models/job-items';
import {
  SliderItem,
  SliderItemKind,
} from 'src/components/base-components/slider-view/slider-view.component';
import { Moment } from 'moment';
import * as moment from 'moment';

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
