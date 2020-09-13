import { Component, OnInit } from '@angular/core';
import { AdminService } from 'src/services/admin_service';
import { Router } from '@angular/router';

@Component({
  selector: 'app-dashboard',
  templateUrl: './dashboard.component.html',
  styleUrls: ['./dashboard.component.styl'],
})
export class DashboardComponent implements OnInit {
  constructor(private adminService: AdminService, private router: Router) {}

  ngOnInit(): void {
    this.adminService.isServerInitialized().then((v) => {
      if (!v) {
        this.router.navigate(['/admin', 'init-db']);
      }
    });
  }
}
