import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { Observable, of } from 'rxjs';
import { endpoints } from 'src/environments/endpoints';
import { catchError } from 'rxjs/operators';
import { ApiService } from './api_service';

@Injectable({ providedIn: 'root' })
export class AdminService {
  public constructor(private httpClient: HttpClient, private api: ApiService) {}

  public isServerInitialized(): Observable<boolean> {
    return this.api.admin.getIsServerInitialized();
  }

  public initializeServer(
    username: string,
    password: string
  ): Observable<void> {
    return this.api.admin.initializeServer(username, password);
  }
}
