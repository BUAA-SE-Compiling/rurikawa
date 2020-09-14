import { Component, OnInit } from '@angular/core';
import { AdminService } from 'src/services/admin_service';
import { Router } from '@angular/router';
import { HttpErrorResponse } from '@angular/common/http';

@Component({
  selector: 'app-init-database',
  templateUrl: './init-database.component.html',
  styleUrls: ['./init-database.component.styl'],
})
export class InitDatabaseComponent implements OnInit {
  constructor(private adminService: AdminService, private router: Router) {}

  username: string = '';
  password: string = '';
  message: string | undefined;

  warnUsername: boolean = false;
  warnPassword: boolean = false;

  loading: boolean = false;

  proceed() {
    this.warnUsername = false;
    this.warnPassword = false;
    this.message = undefined;
    if (!this.username) {
      this.warnUsername = true;
      this.message = '请填写用户名';
      return;
    }
    if (!this.password) {
      this.warnPassword = true;
      this.message = '请填写密码';
      return;
    }
    this.loading = true;

    this.adminService.initializeServer(this.username, this.password).subscribe({
      next: () => {
        this.router.navigate(['/admin']);
      },
      error: (e) => {
        if (e instanceof HttpErrorResponse) {
          this.message = e.message;
        }
        this.loading = false;
      },
    });
  }

  ngOnInit(): void {
    this.adminService.isServerInitialized().subscribe({
      next: (v) => {
        if (v) {
          console.warn('Already initialzed! redirecting');
          this.router.navigate(['/admin']);
        }
      },
    });
  }
}
