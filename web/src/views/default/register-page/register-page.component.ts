import { Component, OnInit } from '@angular/core';
import arrowLeft from '@iconify/icons-carbon/arrow-left';
import { Location } from '@angular/common';
import { AccountService } from 'src/services/account_service';
import { HttpErrorResponse } from '@angular/common/http';
import { Router } from '@angular/router';

@Component({
  selector: 'app-register-page',
  templateUrl: './register-page.component.html',
  styleUrls: ['./register-page.component.less'],
})
export class RegisterPageComponent implements OnInit {
  constructor(
    private location: Location,
    private router: Router,
    private accountService: AccountService
  ) {}

  leftIcon = arrowLeft;

  username: string = '';
  password: string = '';
  usernameMessage: string | undefined;
  passwordMessage: string | undefined;

  back() {
    this.location.back();
  }

  register() {
    this.usernameMessage = undefined;
    this.passwordMessage = undefined;

    this.accountService
      .registerAndLogin(this.username, this.password)
      .subscribe({
        next: (res) => this.router.navigate(['/dashboard']),
        error: (e: HttpErrorResponse) => {
          console.warn(e);
          if (e.error?.err) {
            switch (e.error.err) {
              case 'username_not_unique':
                this.usernameMessage = '用户名已被占用';
                break;
              default:
                this.passwordMessage = '未知错误: ' + e.error.err;
                break;
            }
          } else {
            this.passwordMessage = '未知错误: ' + JSON.stringify(e);
          }
        },
      });
  }

  ngOnInit(): void {}
}
