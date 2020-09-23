import { Component, OnInit } from '@angular/core';
import {
  Job,
  dashboardTypeToSlider,
  TestResultKind,
} from 'src/models/job-items';
import { groupBy, mapValues, toPairs, bindKey } from 'lodash';
import {
  SliderItem,
  SliderItemKind,
} from 'src/components/base-components/slider-view/slider-view.component';
import { ActivatedRoute } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';

import BranchIcon from '@iconify/icons-mdi/source-branch';
import RepoIcon from '@iconify/icons-mdi/git';
import CommitIcon from '@iconify/icons-mdi/source-commit';
import { TestSuiteAndJobCache } from 'src/services/test_suite_cacher';

@Component({
  selector: 'app-job-view',
  templateUrl: './job-view.component.html',
  styleUrls: ['./job-view.component.styl'],
})
export class JobViewComponent implements OnInit {
  constructor(
    private route: ActivatedRoute,
    private service: TestSuiteAndJobCache
  ) {}

  readonly branchIcon = BranchIcon;
  readonly repoIcon = RepoIcon;
  readonly commitIcon = CommitIcon;

  id: string;

  job?: Job = undefined;

  get isFinished() {
    return (
      this.job &&
      this.job.stage === 'Finished' &&
      this.job.resultKind === 'Accepted'
    );
  }

  get branch() {
    return this.job?.branch || 'HEAD';
  }

  get revision() {
    return this.job?.revision.substring(0, 8) || '???';
  }

  titleNumberBrief() {
    if (!this.job) {
      return 'Loading';
    }
    if (this.job.stage !== 'Finished') {
      return this.job.stage;
    }
    if (this.job.resultKind !== 'Accepted') {
      return this.job.resultKind;
    }

    let totalCnt = 0;
    let acCnt = 0;

    // tslint:disable-next-line: forin
    for (let idx in this.job.results) {
      let res = this.job.results[idx];
      totalCnt++;
      if (res.kind === 'Accepted') {
        acCnt++;
      }
    }
    return `${acCnt}/${totalCnt}`;
  }

  briefSlider(): SliderItem[] {
    let res = mapValues(
      groupBy(this.job.results, (result) => dashboardTypeToSlider(result.kind)),
      (i) => i.length
    );
    return toPairs(res).map(([x, y]) => {
      return { kind: x as SliderItemKind, num: y };
    });
  }

  numberBrief(): string {
    return toPairs(
      mapValues(
        groupBy(this.job.results, (result) => result.kind),
        (i) => i.length
      )
    )
      .map((x) => `${x[1]} ${x[0]}`)
      .join(', ');
  }

  results() {
    if (!this.job) {
      return [];
    }
    return toPairs(this.job.results).sort(([ax, ay], [bx, by]) => {
      if (ay.kind !== by.kind) {
        if (ay.kind === 'Accepted') {
          return 1;
        } else if (by.kind === 'Accepted') {
          return -1;
        } else {
          return ay.kind.localeCompare(by.kind);
        }
      } else {
        return ax.localeCompare(bx);
      }
    });
  }

  fetchJob() {
    this.service.getJob(this.id, false, true).subscribe({
      next: (v) => {
        this.job = v;
      },
    });
  }

  ngOnInit(): void {
    this.route.paramMap.subscribe({
      next: (v) => {
        this.id = v.get('id');
        this.fetchJob();
      },
    });
  }
}
