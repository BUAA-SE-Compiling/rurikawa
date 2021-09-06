import { Component, OnInit, OnDestroy } from '@angular/core';
import { Profile } from 'src/models/server-types';
import { HttpErrorResponse } from '@angular/common/http';
import { tap } from 'rxjs/operators';
import { AccountService } from 'src/services/account_service';
import { TitleService } from 'src/services/title_service';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-settings-view',
  templateUrl: './settings-view.component.html',
  styleUrls: ['./settings-view.component.less'],
})
export class SettingsViewComponent implements OnInit, OnDestroy {
  constructor(
    private api: ApiService,
    private accountService: AccountService,
    private title: TitleService
  ) {}
  password = { old: '', new: '' };
  passwordMessage = undefined;

  profile: Profile | undefined = undefined;

  loading = false;
  sending = false;
  profileMessage = undefined;

  initProfile() {
    return this.api.profile.init(this.accountService.Username);
  }

  pullProfile(retry: boolean = false) {
    this.loading = true;
    console.log(this.accountService.Username);
    return this.api.profile.get(this.accountService.Username).pipe(
      tap({
        next: (p) => {
          p.email = p.email ?? '';
          p.studentId = p.studentId ?? '';
          this.profile = p;
          this.loading = false;
        },
        error: (e) => {
          if (e instanceof HttpErrorResponse) {
            if (e.status === 404 && !retry) {
              this.initProfile().subscribe({
                next: () => this.pullProfile(true).subscribe(),
                error: (err) => {
                  console.error(err);
                  this.loading = false;
                },
              });
            }
          }
        },
      })
    );
  }

  updateProfile() {
    if (this.sending) {
      return;
    }
    if (this.profile === undefined) {
      this.profileMessage = '你没填信息';
      return;
    }
    this.sending = true;
    this.profileMessage = undefined;
    this.api.profile.put(this.accountService.Username, this.profile).subscribe({
      next: () => {
        this.sending = false;
      },
      error: (e) => {
        this.sending = false;
        if (e instanceof HttpErrorResponse) {
          this.profileMessage = e.message;
        }
      },
    });
  }

  updatePassword() {
    if (this.sending) {
      return;
    }
    if (this.password.old === '' || this.password.new === '') {
      this.passwordMessage = '你没填密码';
      return;
    }
    this.sending = true;
    this.passwordMessage = undefined;
    this.api.account
      .editPassword(this.password.old, this.password.new)
      .subscribe({
        next: () => {
          this.sending = false;
        },
        error: (e) => {
          this.sending = false;
          if (e instanceof HttpErrorResponse) {
            if (e.status === 404) {
              this.passwordMessage = '找不到用户';
            } else if (e.status === 400) {
              this.passwordMessage = '密码错误';
            } else {
              this.passwordMessage = e.message;
            }
          } else {
            console.error(e);
          }
        },
      });
  }

  ngOnInit(): void {
    this.pullProfile().subscribe();
    this.title.setTitle('设置 - Rurikawa');
  }

  ngOnDestroy() {
    this.title.revertTitle();
  }
}
