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

  suite?: TestSuite[];
  judgerStat?: { count: number; connected: number; running: number };

  navigateToSuite(id: string) {
    this.router.navigate(['admin', 'suite', id]);
  }

  fetchTestSuites() {
    this.httpClient
      .get<TestSuite[]>(
        environment.endpointBase() + endpoints.testSuite.query,
        {
          params: { take: '20' },
        }
      )
      .subscribe({
        next: (v) => (this.suite = v),
      });
  }
  fetchJudgerStat() {
    this.httpClient
      .get<any>(environment.endpointBase() + endpoints.admin.getJudgerStat)
      .subscribe({
        next: (v) => (this.judgerStat = v),
      });
  }

  ngOnInit(): void {
    this.adminService.isServerInitialized().subscribe((v) => {
      if (!v) {
        this.router.navigate(['/admin', 'init-db']);
      }
      this.fetchTestSuites();
      this.fetchJudgerStat();
    });
  }
}
