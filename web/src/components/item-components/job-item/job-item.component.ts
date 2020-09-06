import { Component, OnInit, Input } from '@angular/core';
import { JobItem, dashboardTypeToSlider } from 'src/models/job-items';
import { SliderItem } from 'src/components/base-components/slider-view/slider-view.component';
import * as moment from 'moment';

@Component({
  selector: 'app-job-item',
  templateUrl: './job-item.component.html',
  styleUrls: ['./job-item.component.styl'],
})
export class JobItemComponent implements OnInit {
  @Input() item: JobItem;

  constructor() {}

  public get slider(): SliderItem[] {
    return this.item.status.map<SliderItem>((st) => {
      return {
        kind: dashboardTypeToSlider(st.status),
        num: st.cnt,
      };
    });
  }

  public get timeString(): string {
    return moment(this.item.time).format('YYYY-MM-DD hh:mm');
  }

  ngOnInit(): void {}
}
