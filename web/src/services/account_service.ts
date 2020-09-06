import { Injectable } from '@angular/core';
import {
  HttpClient,
  HttpInterceptor,
  HttpRequest,
  HttpHandler,
  HttpEvent,
  HttpErrorResponse,
} from '@angular/common/http';
import { Observable } from 'rxjs';
import 'rxjs/operators';
import { environment } from 'src/environments/environment';
import { catchError, map, tap } from 'rxjs/operators';

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

  public login(username: string, password: string) {
    this.httpClient
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
        tap((resp) => {
          this.oauthResponse = resp;
        })
      );
  }

  public get Token() {
    return this.oauthResponse?.tokenType + this.oauthResponse?.accessToken;
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
