import { Injectable } from '@angular/core';
import LruCache from 'lru-cache';
import { TestSuite } from 'src/models/server-types';
import { Job } from 'src/models/job-items';

/**
 * A central hub for all test suite and job fetching
 */
@Injectable()
export class TestSuiteAndJobCache {
  private testSuiteCache: LruCache<string, TestSuite>;
  private jobCache: LruCache<string, Job>;

  public fetchTestSuite(id: string) {}
  public fetchJob(id: string) {}
}
