import { Component, OnInit, Input } from '@angular/core';
import {
  JobItem,
  dashboardTypeToSlider,
  Job,
  JobToJobItem,
} from 'src/models/job-items';
import { SliderItem } from 'src/components/base-components/slider-view/slider-view.component';
import { Dayjs } from 'dayjs';
import { TestSuite } from 'src/models/server-types';
import branchIcon from '@iconify/icons-carbon/branch';
import userIcon from '@iconify/icons-carbon/user';

@Component({
  selector: 'app-job-item',
  templateUrl: './job-item.component.html',
  styleUrls: ['./job-item.component.less'],
})
export class JobItemComponent implements OnInit {
  @Input() job: Job;
  @Input() testSuite?: TestSuite;
  @Input() compact: boolean = false;
  @Input() showUsername: boolean = false;

  readonly branchIcon = branchIcon;
  readonly userIcon = userIcon;

  constructor() {}

  public get item(): JobItem {
    return JobToJobItem(this.job, this.testSuite);
  }

  public get slider(): SliderItem[] {
    return this.item.status.map<SliderItem>((st) => {
      return {
        kind: dashboardTypeToSlider(st.status),
        num: st.cnt,
      };
    });
  }

  public get timeString(): string {
    return this.item.time.local().format('YYYY-MM-DD HH:mm');
  }

  ngOnInit(): void {}
}
