import { Injectable } from '@angular/core';
import {
  HttpClient,
  HttpInterceptor,
  HttpRequest,
  HttpHandler,
  HttpEvent,
  HttpErrorResponse,
} from '@angular/common/http';
import { Observable, observable } from 'rxjs';
import 'rxjs/operators';
import { environment } from 'src/environments/environment';
import { catchError, map, tap } from 'rxjs/operators';
import { setUncaughtExceptionCaptureCallback } from 'process';

interface OAuth2Login {
  grantType: string;
  scope: string;
  clientId: string;
  clientSecret: string;
  [key: string]: any;
}

interface OAuth2Response {
  accessToken: string;
  tokenType: string;
  expiresIn?: number;
  refreshToken?: string;
  scope?: string;
}

@Injectable({ providedIn: 'root' })
export class AccountService {
  constructor(private httpClient: HttpClient) {}

  private oauthResponse: OAuth2Response | undefined = undefined;
  private username: string | undefined;

  public login(username: string, password: string) {
    return this.httpClient
      .post<OAuth2Response>(
        environment.endpointBase + '/api/v1/account/login',
        {
          grantType: 'password',
          scope: '',
          clientId: 'web',
          clientSecret: '',
          username,
          password,
        }
      )
      .pipe(
        tap({
          next: (resp) => {
            this.oauthResponse = resp;
            this.username = username;
          },
        })
      );
  }

  public registerAndLogin(username: string, password: string) {
    return new Observable<OAuth2Response>((sub) => {
      this.httpClient
        .post<void>(environment.endpointBase + '/api/v1/account/register', {
          username,
          password,
        })
        .subscribe({
          next: (_) => {
            this.login(username, password).subscribe({
              next: (result) => sub.next(result),
              error: (err) => sub.error(err),
            });
          },
          error: (err) => sub.error(err),
        });
    });
  }

  public get Token() {
    return this.oauthResponse?.tokenType + this.oauthResponse?.accessToken;
  }

  public get UserName() {
    return this.username;
  }

  public get isLoggedIn() {
    return this.oauthResponse !== undefined;
  }
}

@Injectable({ providedIn: 'root' })
export class LoginInformationInterceptor implements HttpInterceptor {
  constructor(private account: AccountService) {}

  intercept(
    req: HttpRequest<any>,
    next: HttpHandler
  ): Observable<HttpEvent<any>> {
    if (this.account.Token !== undefined) {
      req.headers.append('Authorization', this.account.Token);
    }
    return next.handle(req).pipe(
      catchError((e, c) => {
        if (e instanceof HttpErrorResponse) {
          if (e.status === 401) {
          }
        }
        return c;
      })
    );
  }
}
