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
import { TestSuite, NewJobMessage } from 'src/models/server-types';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { flatMap } from 'lodash';
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

@Component({
  selector: 'app-test-suite-view',
  templateUrl: './test-suite-view.component.html',
  styleUrls: ['./test-suite-view.component.styl'],
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
export class TestSuiteViewComponent implements OnInit {
  constructor(
    private route: ActivatedRoute,
    private httpClient: HttpClient,
    private service: TestSuiteAndJobCache,
    private router: Router
  ) {}

  readonly repoIcon = RepoIcon;
  readonly branchIcon = BranchIcon;
  readonly downArrowIcon = DownArrowIcon;
  readonly upArrowIcon = UpArrowIcon;

  repo: string = '';
  branch: string = '';

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

  get items(): JobItem[] | undefined {
    return this.jobs?.map(jobToJobItem);
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

  fetchTestSuite(id: string) {
    this.httpClient
      .get<TestSuite>(environment.endpointBase() + endpoints.testSuite.get(id))
      .subscribe({
        next: (suite) => {
          this.suite = suite;
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
      },
    });
  }

  private initSubmit() {
    if (this.repo === '' && this.jobs && this.jobs.length > 0) {
      this.repo = this.jobs[0].repo;
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
      this.repoMessage = '请填写仓库地址！';
      return;
    }
    this.submittingTest = true;

    if (!branch) {
      branch = undefined;
    }

    let tests = flatMap(this.suite.testGroups, (v) => v);

    let newJobMsg: NewJobMessage = {
      repo,
      ref: branch,
      testSuite: this.suite.id,
      tests,
    };

    this.httpClient
      .post(environment.endpointBase() + endpoints.job.new, newJobMsg, {
        responseType: 'text',
      })
      .subscribe({
        next: (id) => {
          this.submittingTest = false;
          this.loadFirst();
        },
        error: (e) => {
          this.submittingTest = false;
          if (e instanceof HttpErrorResponse) {
            this.repoMessage = e.message;
          }
        },
      });
  }

  ngOnInit(): void {
    this.route.paramMap.subscribe({
      next: (map) => {
        this.id = map.get('id');
        this.fetchTestSuite(this.id);
      },
    });
  }
}
