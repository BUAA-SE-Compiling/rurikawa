import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { Observable } from 'rxjs';
import { endpoints } from 'src/environments/endpoints';

@Injectable({ providedIn: 'root' })
export class AdminService {
  public constructor(private httpClient: HttpClient) {}

  public async isServerInitialized(): Promise<boolean> {
    try {
      let res = await this.httpClient
        .get<boolean>(environment.endpointBase + endpoints.admin.readInitStatus)
        .toPromise();
      return res;
    } catch (e) {
      return false;
    }
  }

  public initializeServer(
    username: string,
    password: string
  ): Observable<void> {
    return this.httpClient.post<void>(
      environment.endpointBase + endpoints.admin.setInitAccount,
      {
        username,
        password,
      }
    );
  }
}
