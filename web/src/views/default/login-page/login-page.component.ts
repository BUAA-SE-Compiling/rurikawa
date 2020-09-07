import { Component, OnInit } from '@angular/core';
import arrowLeft from '@iconify/icons-carbon/arrow-left';
import { Location, JsonPipe } from '@angular/common';
import { AccountService } from 'src/services/account_service';
import { Router } from '@angular/router';
import { HttpErrorResponse } from '@angular/common/http';

@Component({
  selector: 'app-login-page',
  templateUrl: './login-page.component.html',
  styleUrls: ['./login-page.component.styl'],
})
export class LoginPageComponent implements OnInit {
  constructor(
    public location: Location,
    public router: Router,
    private accountService: AccountService
  ) {}

  leftIcon = arrowLeft;

  username: string = '';
  password: string = '';
  message: string | undefined;

  warnUsername: boolean = false;
  warnPassword: boolean = false;

  login() {
    this.message = undefined;
    this.warnPassword = false;
    this.accountService.login(this.username, this.password).subscribe({
      next: (res) => this.router.navigate(['/dashboard']),
      error: (e: HttpErrorResponse) => {
        if (e.error?.err) {
          switch (e.error.err) {
            case 'invalid_login_info':
              this.message = '用户名或密码不正确';
              break;
            default:
              this.message = '未知错误: ' + e.error.err;
              break;
          }
        } else {
          this.message = '未知错误: ' + JSON.stringify(e);
        }
        this.warnPassword = true;
      },
    });
  }

  back() {
    this.location.back();
  }
  ngOnInit(): void {}
}
