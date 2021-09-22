import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import {
  JudgerEntry,
  JudgerStatus,
  QueueStatus,
} from 'src/models/server-types';
import dayjs, { Dayjs } from 'dayjs';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { Subject, Observable, BehaviorSubject, Subscription } from 'rxjs';
import { ApiService } from './api_service';

const JUDGER_STATUS_VALID_PERIOD = 120;

@Injectable({ providedIn: 'root' })
export class JudgerStatusService {
  constructor(private httpClient: HttpClient, private apiService: ApiService) {}

  private updating: boolean;
  private lastData?: { status: JudgerStatus; queue: QueueStatus; time: Dayjs };
  private subject = new BehaviorSubject<JudgerStatus>(undefined);

  private queueSubject = new BehaviorSubject<QueueStatus>(undefined);
  private subscribeCount = 0;

  get data(): JudgerStatus | undefined {
    if (this.lastData === undefined) {
      this.updateData().then(() => {});
    }
    return this.lastData?.status;
  }

  async updateData() {
    if (this.updating) {
      return;
    }
    this.updating = true;
    let data = this.apiService.status.judger().toPromise();
    let queueData = this.apiService.status.queue().toPromise();

    let [status, queueStatus] = await Promise.all([data, queueData]);
    this.lastData = {
      status,
      queue: queueStatus,
      time: dayjs(),
    };

    this.subject.next(status);
    this.queueSubject.next(queueStatus);
    this.updating = false;
  }

  async triggerUpdate() {
    if (
      this.lastData === undefined ||
      this.lastData.time.add(JUDGER_STATUS_VALID_PERIOD, 's').isBefore(dayjs())
    ) {
      await this.updateData();
    }
  }

  assembly: string | undefined;
  public async getAssembly(): Promise<string> {
    if (this.assembly != undefined) return this.assembly;
    let assembly = await this.apiService.status.assembly().toPromise();
    this.assembly = assembly;
    return assembly;
  }

  public subscribeData(action: (JudgerStatus) => void): Subscription {
    return this.subject.subscribe(action);
  }
  public subscribeQueueData(action: (QueueStatus) => void): Subscription {
    return this.queueSubject.subscribe(action);
  }
}
