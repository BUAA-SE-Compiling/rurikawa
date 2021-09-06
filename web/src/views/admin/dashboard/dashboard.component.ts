import { Component, OnInit } from '@angular/core';
import { AdminService } from 'src/services/admin_service';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { TestSuite, JudgerStatus } from 'src/models/server-types';
import { JudgerStatusService } from 'src/services/judger_status_service';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-dashboard',
  templateUrl: './dashboard.component.html',
  styleUrls: ['./dashboard.component.less'],
})
export class DashboardComponent implements OnInit {
  constructor(
    private adminService: AdminService,
    private router: Router,
    private api: ApiService,
    private judgerStatusService: JudgerStatusService
  ) {}

  suite?: TestSuite[];
  judgerStat?: JudgerStatus;

  navigateToSuite(id: string) {
    this.router.navigate(['admin', 'suite', id]);
  }

  fetchTestSuites() {
    this.api.testSuite.query({ take: 20 }).subscribe({
      next: (v) => (this.suite = v),
    });
  }

  fetchJudgerStat() {
    this.judgerStatusService.getData().then((v) => (this.judgerStat = v));
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
