import { Component, OnInit } from '@angular/core';
import { Profile } from 'src/models/server-types';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { tap } from 'rxjs/operators';
import { AccountService } from 'src/services/account_service';

@Component({
  selector: 'app-settings-view',
  templateUrl: './settings-view.component.html',
  styleUrls: ['./settings-view.component.styl'],
})
export class SettingsViewComponent implements OnInit {
  constructor(
    private httpClient: HttpClient,
    private accountService: AccountService
  ) {}

  profile: Profile | undefined = undefined;

  loading = false;
  sending = false;

  initProfile() {
    return this.httpClient.post(
      environment.endpointBase() +
        endpoints.profile.init(this.accountService.Username),
      undefined,
      { responseType: 'text' }
    );
  }

  pullProfile(retry: boolean = false) {
    this.loading = true;
    console.log(this.accountService.Username);
    return this.httpClient
      .get<Profile>(
        environment.endpointBase() +
          endpoints.profile.get(this.accountService.Username)
      )
      .pipe(
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
    if (this.profile === undefined || this.sending) {
      return;
    }
    this.sending = true;
    this.httpClient
      .put(
        environment.endpointBase() +
          endpoints.profile.get(this.accountService.Username),
        this.profile
      )
      .subscribe({
        next: () => {
          this.sending = false;
        },
        error: (e) => {
          this.sending = false;
          console.error(e);
        },
      });
  }

  ngOnInit(): void {
    this.pullProfile().subscribe();
  }
}
