import { Component, OnInit } from '@angular/core';
import { AdminService } from 'src/services/admin_service';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { TestSuite } from 'src/models/server-types';

@Component({
  selector: 'app-dashboard',
  templateUrl: './dashboard.component.html',
  styleUrls: ['./dashboard.component.styl'],
})
export class DashboardComponent implements OnInit {
  constructor(
    private adminService: AdminService,
    private router: Router,
    private httpClient: HttpClient
  ) {}

  suite: TestSuite[];

  fetchTestSuites() {
    this.httpClient
      .get<TestSuite[]>(environment.endpointBase + endpoints.testSuite.query, {
        params: { take: '20' },
      })
      .subscribe({
        next: (v) => (this.suite = v),
      });
  }

  ngOnInit(): void {
    this.adminService.isServerInitialized().subscribe((v) => {
      if (!v) {
        this.router.navigate(['/admin', 'init-db']);
      }
      this.fetchTestSuites();
    });
  }
}
