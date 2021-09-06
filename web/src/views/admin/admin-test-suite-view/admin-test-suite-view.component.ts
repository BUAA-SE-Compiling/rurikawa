import { Component, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { TestSuiteAndJobCache } from 'src/services/test_suite_cacher';
import {
  HttpErrorResponse,
  HttpClient,
  HttpEventType,
} from '@angular/common/http';
import { TestSuite } from 'src/models/server-types';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-admin-test-suite-view',
  templateUrl: './admin-test-suite-view.component.html',
  styleUrls: ['./admin-test-suite-view.component.less'],
})
export class AdminTestSuiteViewComponent implements OnInit {
  constructor(
    private route: ActivatedRoute,
    private testSuiteService: TestSuiteAndJobCache,
    private router: Router,
    private httpClient: HttpClient,
    private api: ApiService
  ) {}

  id: string;
  suite?: TestSuite;

  togglePublic() {
    this.api.testSuite.setVisibility(this.id, !this.suite?.isPublic).subscribe({
      next: () => {
        if (this.suite) {
          this.suite.isPublic = !this.suite.isPublic;
        }
      },
      error: (e) => console.error(e),
    });
  }

  getUploadFileFunction() {
    return (files, started, progress, finished, aborted) => {
      this.uploadFile(
        this.httpClient,
        files,
        started,
        progress,
        finished,
        aborted
      );
    };
  }

  uploadFile(
    http: HttpClient,
    files: File[],
    started: () => void,
    progress: (progress: number) => void,
    finished: (succeed: boolean) => void,
    aborted: () => void
  ) {
    if (files.length !== 1) {
      console.error('There must be exactly 1 file!');
      finished(false);
      return;
    }
    this.api.testSuite.setFile(this.id, files[0]).subscribe({
      next: (ev) => {
        if (ev.type === HttpEventType.UploadProgress) {
          if (ev.total !== undefined) {
            progress(ev.loaded / ev.total);
          }
        } else if (ev.type === HttpEventType.Response) {
          finished(ev.status < 300);
          if (ev.status < 300) {
            this.suite = JSON.parse(ev.body);
          }
        }
      },
      error: (e) => {
        finished(false);
      },
    });
    started();
  }

  // HACK: these methods are dirty
  async dumpJobs(): Promise<void> {
    let code = await this.api.admin.getCode().toPromise();
    console.log('code: ', code);
    window.open(
      environment.endpointBase() +
        endpoints.admin.dumpSuiteJobs(this.id) +
        '?auth=' +
        code,
      'blank'
    );
  }

  async dumpAllJobs(): Promise<void> {
    let code = await this.api.admin.getCode().toPromise();
    console.log('code: ', code);
    window.open(
      environment.endpointBase() +
        endpoints.admin.dumpSuiteAllJobs(this.id) +
        '?auth=' +
        code,
      'blank'
    );
  }

  removeSuite() {
    this.api.testSuite.remove(this.id).subscribe({
      next: () => {
        this.router.navigate(['admin']);
      },
    });
  }

  ngOnInit(): void {
    this.route.paramMap.subscribe((n) => {
      this.id = n.get('id');
      this.testSuiteService.fetchTestSuite(this.id).subscribe({
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
    });
  }
}
