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

@Component({
  selector: 'app-job-view',
  templateUrl: './job-view.component.html',
  styleUrls: ['./job-view.component.styl'],
})
export class JobViewComponent implements OnInit {
  constructor(private route: ActivatedRoute, private httpClient: HttpClient) {}

  id: string;

  job: Job = {
    id: '1abcdefghjklm',
    account: 'rynco',
    repo: 'https://github.com/BUAA-SE-Compiling/natrium',
    branch: 'master',
    testSuite: '1nopqrstvwxyz',
    tests: ['succ'],
    stage: 'Finished',
    resultKind: 'Accepted',
    resultMessage: undefined,
    results: {
      succ: {
        kind: 'Accepted',
        resultFileId: 'tests/1234656',
      },
      succ2: {
        kind: 'WrongAnswer',
        resultFileId: 'tests/1234657',
      },
      succ3: {
        kind: 'Accepted',
        resultFileId: 'tests/1234658',
      },
      succ5: {
        kind: 'TimeLimitExceeded',
        resultFileId: 'tests/1234658',
      },
      succ12: {
        kind: 'MemoryLimitExceeded',
        resultFileId: 'tests/1234658',
      },
      black: {
        kind: 'Accepted',
        resultFileId: 'tests/1234658',
      },
    },
  };

  titleNumberBrief() {
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
    this.httpClient
      .get<Job>(
        environment.endpointBase + endpoints.job.get.replace(':id', this.id)
      )
      .subscribe({
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
