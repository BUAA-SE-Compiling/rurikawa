import { Injectable } from '@angular/core';
import {
  HttpClient,
  HttpInterceptor,
  HttpRequest,
  HttpHandler,
  HttpEvent,
} from '@angular/common/http';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class AccountService {
  constructor(private httpClient: HttpClient) {}

  private bearerToken: string | undefined = undefined;
  public get BearerToken() {
    return this.bearerToken;
  }

  public get isLoggedIn() {
    return this.bearerToken !== undefined;
  }
}

@Injectable({ providedIn: 'root' })
export class LoginInformationInterceptor implements HttpInterceptor {
  constructor(private account: AccountService) {}

  intercept(
    req: HttpRequest<any>,
    next: HttpHandler
  ): Observable<HttpEvent<any>> {
    if (this.account.BearerToken !== undefined) {
      req.headers.append('Authorization', 'Bearer ' + this.account.BearerToken);
    }
    return next.handle(req);
  }
}
