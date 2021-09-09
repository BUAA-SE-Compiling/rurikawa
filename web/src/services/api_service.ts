import { HttpClient } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable, of } from 'rxjs';
import { catchError } from 'rxjs/operators';
import { endpoints } from 'src/environments/endpoints';
import { environment } from 'src/environments/environment';
import { Job } from 'src/models/job-items';
import {
  Announcement,
  DashboardItem,
  JudgerStatus,
  NewJobMessage,
  Profile,
  TestSuite,
} from 'src/models/server-types';

const endpointBase = environment.endpointBase();

@Injectable({ providedIn: 'root' })
export class ApiService {
  constructor(private httpClient: HttpClient) {}

  profile = {
    get: (username: string) =>
      this.httpClient.get<Profile>(
        endpointBase + endpoints.profile.get(username)
      ),

    put: (username: string, profile: Profile) =>
      this.httpClient.put(
        endpointBase + endpoints.profile.put(username),
        profile
      ),

    init: (username: string) =>
      this.httpClient.post(
        endpointBase + endpoints.profile.init(username),
        undefined,
        { responseType: 'text' }
      ),
  };

  account = {
    editPassword: (old: string, new_: string) =>
      this.httpClient.put(endpointBase + endpoints.account.editPassword, {
        original: old,
        new: new_,
      }),

    wsToken: () =>
      this.httpClient.get(endpointBase + endpoints.account.wsToken, {
        responseType: 'text',
      }),
  };

  announcement = {
    get: (id: string) =>
      this.httpClient.get<Announcement>(
        endpointBase + endpoints.announcement.get(id)
      ),

    delete: (id: string) =>
      this.httpClient.delete<Announcement>(
        endpointBase + endpoints.announcement.delete(id)
      ),

    set: (id: string, announcement: Announcement) =>
      this.httpClient.put(
        endpointBase + endpoints.announcement.set(id),
        announcement
      ),

    post: (item: Announcement) =>
      this.httpClient.post(endpointBase + endpoints.announcement.post, item, {
        responseType: 'text',
      }),

    query: (startId: string, count: number, ascending: boolean) =>
      this.httpClient.get<Announcement[]>(
        endpointBase + endpoints.announcement.query,
        {
          params: {
            startId,
            count: count.toString(),
            asc: ascending.toString(),
          },
        }
      ),
  };

  admin = {
    getIsServerInitialized: () =>
      this.httpClient
        .get<boolean>(endpointBase + endpoints.admin.readInitStatus)
        .pipe(catchError((e) => of(false))),

    initializeServer: (username: string, password: string) =>
      this.httpClient.post<void>(
        endpointBase + endpoints.admin.setInitAccount,
        {
          username,
          password,
        }
      ),

    getJudgerStat: () =>
      this.httpClient.get<JudgerStatus>(
        endpointBase + endpoints.admin.getJudgerStat
      ),

    getCode: () =>
      this.httpClient.post(endpointBase + endpoints.admin.getCode, null, {
        responseType: 'text',
      }),

    getJudgerRegisterToken: (
      isSingleUse: boolean,
      tags: string[],
      expiresAt?: Date
    ) =>
      this.httpClient.post(
        endpointBase + endpoints.admin.judgerRegisterToken,
        { isSingleUse, tags, expires: expiresAt },
        { responseType: 'text' }
      ),
  };

  testSuite = {
    query: (params: { take: number }) =>
      this.httpClient.get<TestSuite[]>(
        endpointBase + endpoints.testSuite.query,
        {
          params: { take: params.take.toString() },
        }
      ),

    get: (id: string) =>
      this.httpClient.get<TestSuite>(
        endpointBase + endpoints.testSuite.get(id)
      ),

    getJobs: (id: string, startId: string, take: number, asc: boolean) =>
      this.httpClient.get<Job[]>(
        endpointBase + endpoints.testSuite.getJobs(id),
        {
          params: {
            startId: startId,
            take: take.toString(),
            asc: asc.toString(),
          },
        }
      ),

    setFile: (id: string, file: File) =>
      this.httpClient.put(
        endpointBase + endpoints.testSuite.setFile(id),
        file,
        {
          params: { filename: file.name },
          observe: 'events',
          responseType: 'text',
          reportProgress: true,
        }
      ),

    setVisibility: (id: string, visible: boolean) =>
      this.httpClient.post(
        endpointBase + endpoints.testSuite.setVisibility(id),
        undefined,
        { params: { visible: visible.toString() } }
      ),

    put: (id: string, testSuite: TestSuite) =>
      this.httpClient.put(
        endpointBase + endpoints.testSuite.put(id),
        testSuite
      ),

    remove: (id: string) =>
      this.httpClient.delete(endpointBase + endpoints.testSuite.remove(id)),

    /** Post a new test suite into server */
    post_observeEvents: (files: File) =>
      this.httpClient.post<TestSuite>(
        endpointBase + endpoints.testSuite.post,
        files,
        {
          params: { filename: files.name },
          observe: 'events',
          reportProgress: true,
        }
      ),
    ws: 'tests/ws?token=:token',
  };

  job = {
    get: (id: string) =>
      this.httpClient.get<Job>(endpointBase + endpoints.job.get(id)),
    new: (job: NewJobMessage) =>
      this.httpClient.post(endpointBase + endpoints.job.new, job, {
        responseType: 'text',
      }),
    query: 'job',

    respawn: (id: string) =>
      this.httpClient.post(endpointBase + endpoints.job.respawn(id), null, {
        responseType: 'text',
      }),
  };

  dashboard = {
    get: () => {
      return this.httpClient.get<DashboardItem[]>(
        endpointBase + endpoints.dashboard.get
      );
    },
  };

  public getFile<T>(path: string): Observable<T> {
    return this.httpClient.get<T>(endpointBase + endpoints.file.get(path), {
      headers: { 'bypass-login': 'true' },
    });
  }
}
