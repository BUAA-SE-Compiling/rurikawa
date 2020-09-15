import { Component, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import {
  JobItem,
  Job,
  JobToJobItem as jobToJobItem,
} from 'src/models/job-items';
import repo from '@iconify/icons-carbon/link';
import branch from '@iconify/icons-carbon/branch';
import { TestSuite, NewJobMessage } from 'src/models/server-types';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { flatMap } from 'lodash';

@Component({
  selector: 'app-test-suite-view',
  templateUrl: './test-suite-view.component.html',
  styleUrls: ['./test-suite-view.component.styl'],
})
export class TestSuiteViewComponent implements OnInit {
  constructor(
    private route: ActivatedRoute,
    private httpClient: HttpClient,
    private router: Router
  ) {}

  readonly repoIcon = repo;
  readonly branchIcon = branch;

  repo: string = '';
  branch: string = '';

  repoMessage: string | undefined;
  branchMessage: string | undefined;

  id: string;

  loadingSuite: boolean = false;
  loadingJobs: boolean = false;
  incrementLoadingJobs: boolean = false;

  suite: TestSuite | undefined;

  items: JobItem[] | undefined = undefined;
  jobs: Job[] | undefined = undefined;

  gotoJob(id: string) {
    this.router.navigate(['job', id]);
  }

  fetchTestSuite(id: string) {
    this.httpClient
      .get<TestSuite>(
        environment.endpointBase + endpoints.testSuite.get.replace(':id', id)
      )
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
    this.httpClient
      .get<Job[]>(
        environment.endpointBase +
          endpoints.testSuite.getJobs.replace(':id', id)
      )
      .subscribe({
        next: (jobs) => {
          this.jobs = jobs;
          this.items = jobs.map(jobToJobItem);
        },
      });
  }

  submitTest() {
    this.repoMessage = undefined;
    let branch = this.branch;
    let repo = this.repo;

    if (!repo) {
      this.repoMessage = '请填写仓库地址！';
      return;
    }
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
      .post<string>(environment.endpointBase + endpoints.job.new, newJobMsg)
      .subscribe({
        next: (id) => {
          console.log(id);
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
