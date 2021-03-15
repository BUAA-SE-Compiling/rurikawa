import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { JudgerEntry, JudgerStatus } from 'src/models/server-types';
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
  private lastData?: { status: JudgerStatus; time: Dayjs };
  private subject = new BehaviorSubject<JudgerStatus>(undefined);

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
    console.log('update');
    let data = await this.apiService.admin.getJudgerStat().toPromise();

    this.lastData = {
      status: data,
      time: dayjs(),
    };
    console.log('updated', data);

    this.subject.next(data);
    this.updating = false;
  }

  async getData(): Promise<JudgerStatus> {
    if (
      this.lastData === undefined ||
      this.lastData.time.add(JUDGER_STATUS_VALID_PERIOD, 's').isBefore(dayjs())
    ) {
      await this.updateData();
    }

    return this.lastData.status;
  }

  public subscribeData(action: (JudgerStatus) => void): Subscription {
    return this.subject.subscribe(action);
  }
}
