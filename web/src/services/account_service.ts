import { Injectable } from '@angular/core';
import {
  HttpClient,
  HttpInterceptor,
  HttpRequest,
  HttpHandler,
  HttpEvent,
  HttpErrorResponse,
} from '@angular/common/http';
import { Observable, observable, throwError } from 'rxjs';
import 'rxjs/operators';
import { environment } from 'src/environments/environment';
import { catchError, map, tap, switchMap } from 'rxjs/operators';
import { setUncaughtExceptionCaptureCallback } from 'process';
import {
  CanActivate,
  ActivatedRouteSnapshot,
  RouterStateSnapshot,
  UrlTree,
  Router,
  CanActivateChild,
  CanLoad,
  Route,
} from '@angular/router';
import { endpoints } from 'src/environments/endpoints';

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
  role?: string;
  scope?: string;
}

@Injectable({ providedIn: 'root' })
export class AccountService {
  constructor(private httpClient: HttpClient, private router: Router) {
    this.tryLoadLoginInfo();
  }

  private oauthResponse: OAuth2Response | undefined = undefined;
  private username: string | undefined;

  private attemptedToAccessUri: string | undefined;

  public login(username: string, password: string) {
    return this.httpClient
      .post<OAuth2Response>(
        environment.endpointBase() + endpoints.account.login,
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
            if (this.attemptedToAccessUri !== undefined) {
              this.router.navigateByUrl(this.attemptedToAccessUri);
              this.attemptedToAccessUri = undefined;
            }
            this.saveLoginInfo();
          },
        })
      );
  }

  public loginUsingRefreshToken() {
    if (!this.oauthResponse) {
      return new Observable<OAuth2Response>((sub) =>
        sub.error('not_logged_in')
      );
    } else {
      return this.httpClient
        .post<OAuth2Response>(
          environment.endpointBase() + endpoints.account.login,
          {
            grantType: 'refresh_token',
            scope: '',
            clientId: 'web',
            clientSecret: '',
            refreshToken: this.oauthResponse.refreshToken,
          }
        )
        .pipe(
          tap({
            next: (resp) => {
              this.oauthResponse = resp;
              this.saveLoginInfo();
            },
          })
        );
    }
  }

  public registerAndLogin(username: string, password: string) {
    return new Observable<OAuth2Response>((sub) => {
      this.httpClient
        .post<void>(environment.endpointBase() + endpoints.account.register, {
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

  public logout() {
    this.clearSavedLoginInfo();
    this.oauthResponse = undefined;
    this.username = undefined;
  }

  public saveLoginInfo() {
    window.localStorage.setItem('auth', JSON.stringify(this.oauthResponse));
    window.localStorage.setItem('usr', this.username);
  }

  public tryLoadLoginInfo() {
    let authString = window.localStorage.getItem('auth');
    let usr = window.localStorage.getItem('usr');
    if (authString != null && usr != null) {
      let oauth = JSON.parse(authString);
      this.oauthResponse = oauth;
      this.username = usr;
    }
  }

  public clearSavedLoginInfo() {
    window.localStorage.removeItem('usr');
    window.localStorage.removeItem('auth');
  }

  public async loggedInOrRedirect(
    attempted?: string
  ): Promise<boolean | UrlTree> {
    if (this.isLoggedIn) {
      return true;
    }
    if (attempted !== undefined) {
      this.attemptedToAccessUri = attempted;
    }
    this.router.createUrlTree(['/login']);
  }

  public isInRoles(roles: string[]): boolean {
    return (
      this.isLoggedIn &&
      this.oauthResponse.role &&
      roles.includes(this.oauthResponse.role)
    );
  }

  public async roleOrRedirect(
    roles: string[],
    attempted: string | undefined = undefined,
    to: string[] = ['/login']
  ): Promise<boolean | UrlTree> {
    if (
      this.isLoggedIn &&
      this.oauthResponse.role &&
      roles.includes(this.oauthResponse.role)
    ) {
      return true;
    }
    if (attempted !== undefined) {
      this.attemptedToAccessUri = attempted;
    }
    this.router.createUrlTree(to);
  }

  public get Token() {
    if (this.oauthResponse === undefined) {
      return undefined;
    } else {
      return 'Bearer ' + this.oauthResponse.accessToken;
    }
  }

  public get Username() {
    return this.username;
  }

  public get Role() {
    return this.oauthResponse?.role;
  }

  public get isAdmin() {
    return this.Role === 'Admin' || this.Role === 'Root';
  }

  public get isLoggedIn() {
    return this.oauthResponse !== undefined;
  }
}

@Injectable({ providedIn: 'root' })
export class LoginInformationInterceptor implements HttpInterceptor {
  constructor(private account: AccountService) {}

  refreshTokenRunning: boolean = false;

  intercept(
    req: HttpRequest<any>,
    next: HttpHandler
  ): Observable<HttpEvent<any>> {
    console.error(req.url);
    if (
      req.headers.get('bypass-login') !== null ||
      !req.url.startsWith(environment.endpointBase())
    ) {
      return next.handle(
        req.clone({ headers: req.headers.delete('bypass-login') })
      );
    }
    if (this.account.Token !== undefined) {
      req = req.clone({
        headers: req.headers.set('Authorization', this.account.Token),
      });
    }
    return next.handle(req).pipe(
      catchError((e, c) => {
        if (e instanceof HttpErrorResponse) {
          if (e.status === 401 && !this.refreshTokenRunning) {
            this.refreshTokenRunning = true;
            return this.account.loginUsingRefreshToken().pipe(
              tap(
                () => (this.refreshTokenRunning = false),
                () => (this.refreshTokenRunning = false)
              ),
              switchMap(() => this.intercept(req, next)),
              catchError((e, c) => {
                return throwError(e);
              })
            );
          }
        }
        return throwError(e);
      })
    );
  }
}

@Injectable({
  providedIn: 'root',
})
export class NotLoggedInGuard implements CanActivate, CanActivateChild {
  constructor(private accountService: AccountService) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    return !this.accountService.isLoggedIn;
  }

  canActivateChild(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    return !this.accountService.isLoggedIn;
  }
}

@Injectable({
  providedIn: 'root',
})
export class NotLoggedInRedirectToDashboardGuard
  implements CanActivate, CanActivateChild {
  constructor(private accountService: AccountService, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    if (!this.accountService.isLoggedIn) {
      return true;
    } else {
      return this.router.createUrlTree(['/dashboard']);
    }
  }

  canActivateChild(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    if (!this.accountService.isLoggedIn) {
      return true;
    } else {
      return this.router.createUrlTree(['/dashboard']);
    }
  }
}

@Injectable({
  providedIn: 'root',
})
export class LoginGuard implements CanActivate, CanActivateChild {
  constructor(private accountService: AccountService) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    return this.accountService.loggedInOrRedirect(state.url);
  }

  canActivateChild(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    return this.accountService.loggedInOrRedirect(state.url);
  }
}

@Injectable({
  providedIn: 'root',
})
export class AdminLoginGuard implements CanActivate, CanActivateChild, CanLoad {
  constructor(private accountService: AccountService) {}
  canLoad(route: Route, segments: import('@angular/router').UrlSegment[]) {
    // return this.accountService.isInRoles(['Admin', 'Root']);
    return true;
  }

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    if (state.url === '/admin/init-db') {
      return true;
    } else {
      return this.accountService.roleOrRedirect(['Admin', 'Root'], undefined, [
        '/403',
      ]);
    }
  }

  canActivateChild(route: ActivatedRouteSnapshot, state: RouterStateSnapshot) {
    if (state.url === '/admin/init-db') {
      return true;
    } else {
      return this.accountService.roleOrRedirect(['Admin', 'Root'], undefined, [
        '/403',
      ]);
    }
  }
}
