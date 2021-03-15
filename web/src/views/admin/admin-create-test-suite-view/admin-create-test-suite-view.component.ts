import { Component, OnInit } from '@angular/core';
import { HttpClient, HttpEventType } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { TestSuite } from 'src/models/server-types';
import { Router } from '@angular/router';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-admin-create-test-suite-view',
  templateUrl: './admin-create-test-suite-view.component.html',
  styleUrls: ['./admin-create-test-suite-view.component.styl'],
})
export class AdminCreateTestSuiteViewComponent implements OnInit {
  constructor(private api:ApiService, private router: Router) {}

  testSuite?: TestSuite;

  ngOnInit(): void {}

  getUploadFileFunction() {
    return (files, started, progress, finished, aborted) => {
      this.uploadFile(
        this.api,
        files,
        started,
        progress,
        finished,
        aborted
      );
    };
  }

  uploadFile(
    api:ApiService
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
    api.testSuite.post_observeEvents(files[0])
      .subscribe({
        next: (ev) => {
          if (ev.type === HttpEventType.UploadProgress) {
            if (ev.total !== undefined) {
              progress(ev.loaded / ev.total);
            }
          } else if (ev.type === HttpEventType.Response) {
            finished(ev.status < 300);
            if (ev.status < 300) {
              this.testSuite = ev.body;
              this.router.navigate(['admin', 'suite', this.testSuite.id]);
            }
          }
        },
        error: (e) => {
          finished(false);
        },
      });
    started();
  }
}
