import { Component, OnInit, Input } from '@angular/core';
import { dashboardTypeToSlider } from 'src/models/job-items';
import {
  SliderItem,
  SliderItemKind,
} from 'src/components/base-components/slider-view/slider-view.component';
import { Dayjs} from 'dayjs'
import { DashboardItem } from 'src/models/server-types';
import { mapValues, groupBy, toPairs } from 'lodash';
import { extractTime } from 'src/models/flowsnake';

@Component({
  selector: 'app-dashboard-item-component',
  templateUrl: './dashboard-item-component.component.html',
  styleUrls: ['./dashboard-item-component.component.styl'],
})
export class DashboardItemComponentComponent implements OnInit {
  constructor() {}

  @Input() item: DashboardItem;
  @Input() compact: boolean = false;

  private _slider: SliderItem[] | undefined;

  getSlider(): SliderItem[] {
    if (
      this.item.job === undefined ||
      this.item.job.stage !== 'Finished' ||
      this.item.job.resultKind !== 'Accepted'
    ) {
      return [{ kind: 'disable', num: 1 }];
    }
    let res = mapValues(
      groupBy(this.item.job.results, (result) =>
        dashboardTypeToSlider(result.kind)
      ),
      (i) => i.length
    );
    let res1 = toPairs(res).map(([x, y]) => {
      return { kind: x as SliderItemKind, num: y };
    });
    console.log(res1);
    return res1;
  }

  public get slider(): SliderItem[] {
    return this._slider;
  }

  public get itemCountString(): string {
    if (this.item.job === undefined) {
      return 'N/A';
    } else if (this.item.job.stage !== 'Finished') {
      return this.item.job.stage;
    } else if (this.item.job.resultKind !== 'Accepted') {
      return this.item.job.resultKind;
    } else {
      let totalItems = this._slider.reduce((p, v) => p + v.num, 0);
      let finishedItems = this._slider.reduce(
        (p, v) => (p + v.kind === 'accept' ? v.num : 0),
        0
      );
      return `${finishedItems}/${totalItems}`;
    }
  }

  public get timeString(): string {
    let id = this.item.job?.id;
    if (id === undefined) {
      return '---- --:--';
    }
    return extractTime(id).local().format('YYYY-MM-DD HH:mm');
  }

  ngOnInit(): void {
    this._slider = this.getSlider();
  }
}
