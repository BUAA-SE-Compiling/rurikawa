import { Injectable, NgZone } from '@angular/core';
import * as LruCache from 'quick-lru';
import { TestSuite } from 'src/models/server-types';
import { Job, JobStage } from 'src/models/job-items';
import { HttpClient } from '@angular/common/http';
import { assign } from 'lodash';
import { endpoints } from 'src/environments/endpoints';
import { environment } from 'src/environments/environment';
import { tap } from 'rxjs/operators';
import {
  SubscribeMsg,
  WsApiMsg,
  JobStatusUpdateMsg,
} from 'src/models/ws-types';
import {
  WebSocketSubject,
  WebSocketSubjectConfig,
  webSocket,
} from 'rxjs/webSocket';
import { Observable, of, Subject } from 'rxjs';
import { preserveWhitespacesDefault } from '@angular/compiler';
import { ApiService } from './api_service';

export interface FetchSuiteJobOption {
  suiteId: string;
  startId?: string;
  asc?: boolean;
  take?: number;
  cache?: boolean;
  tracking?: boolean;
}

const fetchSuiteJobDefault: FetchSuiteJobOption = {
  suiteId: '',
  startId: '0000000000000',
  asc: false,
  take: 20,
  cache: true,
  tracking: true,
};

/**
 * A central hub for all test suite and job fetching
 */
@Injectable({ providedIn: 'root' })
export class TestSuiteAndJobCache {
  public constructor(private httpClient: HttpClient, private api: ApiService) {
    this.testSuiteCache = new LruCache({ maxSize: 1000 });
    this.jobCache = new LruCache({
      maxSize: 1000,
      onEviction: (k, v) => {
        this.stopTrackingJob(k);
      },
    });
    this.connectWebsocket();
  }

  private wsTracker: WebSocketSubject<WsApiMsg> | undefined;

  private testSuiteCache: LruCache<string, TestSuite>;
  private jobCache: LruCache<string, Job>;
  private trackingJobs: Set<string> = new Set();

  private getWebsocketToken() {
    return this.api.account.wsToken();
  }

  public connectWebsocket() {
    this.getWebsocketToken().subscribe({
      next: (token) => {
        this.wsTracker = webSocket({
          url:
            environment.websocketBase() +
            endpoints.testSuite.ws.replace(':token', token),
        });
        this.wsTracker.subscribe({
          next: (v) => this.onWebsocketMessage(v),
          error: (e) => {
            this.connectWebsocket();
          },
          complete: () => {
            this.connectWebsocket();
          },
        });
      },
    });
  }

  private onWebsocketMessage(msg: WsApiMsg) {
    switch (msg._t) {
      case 'job_status_s':
        this.updateJobStatus(msg as JobStatusUpdateMsg);
        break;
      case 'judger_status_s':
        break;
      case 'new_job_s':
        break;
      case 'test_output_s':
        break;
    }
  }

  private updateJobStatus(msg: JobStatusUpdateMsg) {
    let job = this.jobCache.get(msg.jobId);
    if (job !== undefined) {
      if (msg.jobResult !== undefined) {
        job.resultKind = msg.jobResult;
      }
      if (msg.stage !== undefined) {
        job.stage = msg.stage;
      }
      if (msg.testResult !== undefined) {
        Object.assign(job.results, msg.testResult);
      }
      if (msg.buildOutputFile !== undefined) {
        job.buildOutputFile = msg.buildOutputFile;
      }
    }
  }

  public cacheJob(job: Job) {
    this.jobCache.set(job.id, job);
  }

  public startTrackingJob(...jobId: string[]) {
    jobId.forEach((id) => this.trackingJobs.add(id));
    let msg: SubscribeMsg = {
      _t: 'sub_c',
      sub: true,
      jobs: jobId,
    };
    this.wsTracker?.next(msg);
  }

  public stopTrackingJob(...jobId: string[]) {
    jobId.forEach((id) => this.trackingJobs.add(id));
    let msg: SubscribeMsg = {
      _t: 'sub_c',
      sub: false,
      jobs: jobId,
    };
    this.wsTracker?.next(msg);
  }

  public getTestSuite(id: string, forceUpdate: boolean = false) {
    if (!forceUpdate) {
      let suite = this.testSuiteCache.get(id);
      if (suite !== undefined) {
        return of(suite);
      }
    }
    return this.fetchTestSuite(id);
  }

  public getJob(
    id: string,
    forceUpdate: boolean = false,
    track: boolean = true
  ) {
    if (!forceUpdate) {
      let job = this.jobCache.get(id);
      if (job !== undefined) {
        return of(job);
      }
    }
    return this.fetchJob(id, track);
  }

  /**
   * fetch jobs from test suite using the following options
   * @param optU User-provided options
   */
  public fetchSuiteJobs(optU: FetchSuiteJobOption) {
    let opt = fetchSuiteJobDefault;
    assign(opt, optU);

    return this.api.testSuite
      .getJobs(opt.suiteId, opt.startId, opt.take, opt.asc)
      .pipe(
        tap({
          next: (jobs) => {
            if (opt.tracking || opt.cache) {
              jobs.forEach((j) => this.cacheJob(j));
            }
            if (opt.tracking) {
              this.startTrackingJob(...jobs.map((j) => j.id));
            }
          },
        })
      );
  }

  public fetchTestSuite(id: string) {
    return this.api.testSuite.get(id).pipe(
      tap({
        next: (v) => {
          this.testSuiteCache.set(v.id, v);
        },
      })
    );
  }

  public fetchJob(id: string, track: boolean) {
    return this.api.job.get(id).pipe(
      tap({
        next: (v) => {
          if (track) {
            this.startTrackingJob(v.id);
          }
          this.cacheJob(v);
        },
      })
    );
  }
}
