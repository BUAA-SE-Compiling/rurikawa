import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { Observable, of } from 'rxjs';
import { endpoints } from 'src/environments/endpoints';
import { catchError } from 'rxjs/operators';

@Injectable({ providedIn: 'root' })
export class AdminService {
  public constructor(private httpClient: HttpClient) {}

  public isServerInitialized(): Observable<boolean> {
    return this.httpClient
      .get<boolean>(environment.endpointBase() + endpoints.admin.readInitStatus)
      .pipe(catchError((e) => of(false)));
  }

  public initializeServer(
    username: string,
    password: string
  ): Observable<void> {
    return this.httpClient.post<void>(
      environment.endpointBase() + endpoints.admin.setInitAccount,
      {
        username,
        password,
      }
    );
  }
}
