import { Component, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { TestSuiteAndJobCache } from 'src/services/test_suite_cacher';
import { HttpErrorResponse, HttpClient } from '@angular/common/http';
import { TestSuite } from 'src/models/server-types';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';

@Component({
  selector: 'app-admin-test-suite-view',
  templateUrl: './admin-test-suite-view.component.html',
  styleUrls: ['./admin-test-suite-view.component.styl'],
})
export class AdminTestSuiteViewComponent implements OnInit {
  constructor(
    private route: ActivatedRoute,
    private testSuiteService: TestSuiteAndJobCache,
    private router: Router,
    private httpClient: HttpClient
  ) {}

  id: string;
  suite?: TestSuite;

  togglePublic() {
    this.httpClient
      .post(
        environment.endpointBase() + endpoints.testSuite.setVisibility(this.id),
        undefined,
        {
          params: {
            visible: this.suite?.isPublic ? 'false' : 'true',
          },
        }
      )
      .subscribe({
        next: () => {
          if (this.suite) {
            this.suite.isPublic = !this.suite.isPublic;
          }
        },
        error: (e) => console.error(e),
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
