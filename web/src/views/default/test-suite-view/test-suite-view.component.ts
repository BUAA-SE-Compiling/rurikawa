import { Component, OnInit, ChangeDetectorRef, OnDestroy } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import {
  JobItem,
  Job,
  JobToJobItem as jobToJobItem,
} from 'src/models/job-items';

import BranchIcon from '@iconify/icons-mdi/source-branch';
import RepoIcon from '@iconify/icons-mdi/git';
import DownArrowIcon from '@iconify/icons-mdi/chevron-down';
import UpArrowIcon from '@iconify/icons-mdi/chevron-up';
import TimeIcon from '@iconify/icons-carbon/timer';
import UserIcon from '@iconify/icons-carbon/user';
import PasswordIcon from '@iconify/icons-carbon/password';
import MemoryIcon from '@iconify/icons-carbon/chart-treemap';
import {
  TestSuite,
  NewJobMessage,
  TestCaseDefinition,
} from 'src/models/server-types';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { flatMap, toPairs } from 'lodash';
import {
  trigger,
  transition,
  style,
  stagger,
  query,
  animate,
} from '@angular/animations';
import { tap } from 'rxjs/operators';
import { TestSuiteAndJobCache } from 'src/services/test_suite_cacher';
import { Subscription } from 'rxjs';
import { TitleService } from 'src/services/title_service';
import {
  errorCodeToDescription,
  errorResponseToDescription,
} from 'src/models/errors';
import { ApiService } from 'src/services/api_service';

const repoParser = /^https:\/\/(?:(.*?)(?::(.*?))?@)?(.+)$/;

@Component({
  selector: 'app-test-suite-view',
  templateUrl: './test-suite-view.component.html',
  styleUrls: ['./test-suite-view.component.less'],
  animations: [
    trigger('staggerLoadJobs', [
      transition('*=>*', [
        query(
          '.test-item:enter',
          [
            style({ opacity: 0 }),
            stagger(10, animate(100, style({ opacity: 1 }))),
          ],
          { optional: true }
        ),
      ]),
    ]),
  ],
})
export class TestSuiteViewComponent implements OnInit, OnDestroy {
  constructor(
    private route: ActivatedRoute,
    private api: ApiService,
    private service: TestSuiteAndJobCache,
    private router: Router,
    private title: TitleService
  ) {}

  readonly repoIcon = RepoIcon;
  readonly branchIcon = BranchIcon;
  readonly downArrowIcon = DownArrowIcon;
  readonly upArrowIcon = UpArrowIcon;
  readonly timeIcon = TimeIcon;
  readonly memoryIcon = MemoryIcon;
  readonly userIcon = UserIcon;
  readonly passwordIcon = PasswordIcon;

  repo: string = '';
  branch: string = '';
  username: string = '';
  password: string = '';

  repoMessage: string | undefined;
  branchMessage: string | undefined;

  id: string;

  loadingSuite: boolean = false;
  loadingJobs: boolean = false;
  incrementLoadingJobs: boolean = false;

  suite: TestSuite | undefined;

  jobs: Job[] | undefined = undefined;

  submittingTest: boolean = false;
  allJobsFinished: boolean = false;

  descCollapsed: boolean = false;

  testGroups: { key: string; values: TestCaseDefinition[] }[];
  usingTestGroup: Set<string> = new Set();

  getTestGroups() {
    if (this.suite === undefined) {
      return [];
    }
    return toPairs(this.suite.testGroups)
      .map(([k, v]) => {
        return {
          key: k,
          values: v,
        };
      })
      .sort((o1, o2) => o1.key.localeCompare(o2.key));
  }

  loadMore() {
    if (this.jobs === undefined || this.jobs.length === 0) {
      this.fetchJobs(this.id).subscribe();
    } else {
      let lastId = this.jobs[this.jobs.length - 1].id;
      this.fetchJobs(this.id, { startId: lastId }).subscribe();
    }
  }

  loadFirst() {
    if (this.jobs === undefined || this.jobs.length === 0) {
      this.fetchJobs(this.id).subscribe();
    } else {
      let firstId = this.jobs[0].id;
      this.fetchJobs(this.id, {
        startId: firstId,
        insertInFront: true,
      }).subscribe();
    }
  }

  gotoJob(id: string) {
    this.router.navigate(['job', id]);
  }

  trackBy(item: JobItem) {
    return item.id;
  }

  determineLastTestGroup() {
    if (this.suite === undefined) {
      return;
    }
    if (this.jobs === undefined || this.jobs.length === 0) {
      // tslint:disable-next-line: forin
      for (let key in this.suite.testGroups) {
        this.usingTestGroup.add(key);
      }
    } else {
      let groupMap = new Map<string, string>();
      // tslint:disable-next-line: forin
      for (let key in this.suite.testGroups) {
        for (let test of this.suite.testGroups[key]) {
          groupMap.set(test.name, key);
        }
      }
      let lastJob = this.jobs[0];
      for (let test of lastJob.tests) {
        const group = groupMap.get(test);
        if (group !== undefined) {
          this.usingTestGroup.add(group);
        }
      }
    }
  }

  changeGroupActivation(name: string, active: boolean) {
    if (active) {
      this.usingTestGroup.add(name);
    } else {
      this.usingTestGroup.delete(name);
    }
  }

  fetchTestSuite(id: string) {
    this.service.getTestSuite(id).subscribe({
      next: (suite) => {
        this.suite = suite;
        this.testGroups = this.getTestGroups();
        this.determineLastTestGroup();

        this.title.setTitle(`${suite.title} - Rurikawa`, 'test-suite');
      },
      error: (e) => {
        if (e instanceof HttpErrorResponse) {
          if (e.status === 404) {
            this.router.navigate(['/404']);
          }
        }
      },
    });
    this.fetchJobs(id).subscribe({
      next: () => {
        if (this.jobs.length > 0) {
          this.descCollapsed = true;
        }
        this.determineLastTestGroup();
      },
    });
  }

  private initSubmit() {
    if (this.repo === '' && this.jobs && this.jobs.length > 0) {
      let repoParseResult = repoParser.exec(this.jobs[0].repo);
      if (repoParseResult == null) {
        this.repo = this.jobs[0].repo;
      } else {
        this.repo = 'https://' + repoParseResult[3];
        this.username = repoParseResult[1] ?? '';
        this.password = repoParseResult[2] ?? '';
      }
      this.branch = this.jobs[0].branch;
    }
  }

  private fetchJobs(
    id: string,
    opt: {
      startId?: string;
      take?: number;
      insertInFront?: boolean;
    } = {}
  ) {
    opt.startId = opt.startId ?? '0000000000000';
    opt.take = opt.take ?? 20;
    opt.insertInFront = opt.insertInFront ?? false;

    return this.service
      .fetchSuiteJobs({
        suiteId: id,
        asc: opt.insertInFront,
        cache: true,
        startId: opt.startId,
        take: opt.take,
        tracking: true,
      })
      .pipe(
        tap({
          next: (jobs) => {
            if (jobs.length < opt.take && !opt.insertInFront) {
              this.allJobsFinished = true;
            }
            if (opt.insertInFront) {
              jobs = jobs.reverse();
            }
            if (this.jobs === undefined || this.jobs.length === 0) {
              this.jobs = jobs;
              this.initSubmit();
            } else if (opt.insertInFront) {
              this.jobs.unshift(...jobs);
            } else {
              this.jobs.push(...jobs);
            }
          },
        })
      );
  }

  submitTest() {
    if (this.submittingTest) {
      return;
    }
    this.repoMessage = undefined;
    let branch = this.branch;
    let repo = this.repo;

    if (!repo) {
      this.repoMessage = '请填写仓库地址';
      return;
    }
    let repoParseResult = repoParser.exec(this.repo);
    if (repoParseResult == null) {
      this.repoMessage = '你提交的大概不是 HTTPS 仓库地址';
      return;
    }
    let rawRepo = repoParseResult[3];
    if (this.username) {
      if (this.password) {
        repo = `https://${this.username}:${this.password}@${rawRepo}`;
      } else {
        repo = `https://${this.username}@${rawRepo}`;
      }
    }

    if (this.usingTestGroup.size === 0) {
      this.repoMessage = '请至少选择一个测试';
      return;
    }

    this.submittingTest = true;

    if (!branch) {
      branch = undefined;
    }

    let tests = flatMap(this.suite.testGroups, (v, k) =>
      this.usingTestGroup.has(k) ? v : []
    ).map((v) => v.name);

    let newJobMsg: NewJobMessage = {
      repo,
      ref: branch,
      testSuite: this.suite.id,
      tests,
    };

    this.api.job.new(newJobMsg).subscribe({
      next: (id) => {
        this.submittingTest = false;
        this.loadFirst();
      },
      error: (e) => {
        this.submittingTest = false;
        if (e instanceof HttpErrorResponse) {
          this.repoMessage = errorResponseToDescription(e);
        }
      },
    });
  }

  ngOnInit(): void {
    this.route.paramMap.subscribe({
      next: (map) => {
        this.id = map.get('id');
        this.fetchTestSuite(this.id);
        this.title.setTitle(`${this.id} - Test Suite - Rurikawa`, 'test-suite');
      },
    });
  }

  ngOnDestroy() {
    this.title.revertTitle('test-suite');
  }
}
