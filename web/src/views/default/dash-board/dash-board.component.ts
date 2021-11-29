import { Component, OnInit, OnDestroy } from '@angular/core';
// import { DashboardItem } from 'src/models/job-items';
import { Router } from '@angular/router';
import { HttpErrorResponse } from '@angular/common/http';
import {
  Announcement,
  DashboardItem,
  JudgerStatus,
  QueueStatus,
} from 'src/models/server-types';
import { TitleService } from 'src/services/title_service';
import { JudgerStatusService } from 'src/services/judger_status_service';
import { Subscription } from 'rxjs';
import { ApiService } from 'src/services/api_service';
import { FLOWSNAKE_MAX as FLOWSNAKE_MAX } from 'src/models/flowsnake';
import { AccountService } from 'src/services/account_service';
import {
  NavbarService,
  NAVBAR_DEFAULT_STYLE,
} from 'src/services/navbar_service';

const DEFAULT_NUM_ITEMS = 20;

@Component({
  selector: 'app-dash-board',
  templateUrl: './dash-board.component.html',
  styleUrls: ['./dash-board.component.less'],
})
export class DashBoardComponent implements OnInit, OnDestroy {
  constructor(
    private router: Router,
    private api: ApiService,
    private title: TitleService,
    public account: AccountService,
    public judgerStatusService: JudgerStatusService,
    navbarService: NavbarService
  ) {
    navbarService.pushStyle(NAVBAR_DEFAULT_STYLE, false);
  }
  loading = true;
  items: DashboardItem[] | undefined = undefined;
  itemsCount: number = 0;
  hasMore = true;

  announcements: Announcement[] | undefined = undefined;

  error: boolean = false;
  errorMessage?: string;

  judgerStat?: JudgerStatus;
  queueStatus?: QueueStatus;
  assemblyInfo?: string;

  judgerSubscription?: Subscription;
  queueSubscription?: Subscription;

  gotoJudgeSuite(id: string) {
    this.router.navigate(['/suite', id]);
  }

  fetchAnnouncements() {
    this.api.announcement.query(FLOWSNAKE_MAX, 3, false).subscribe({
      next: (a) => {
        this.announcements = a;
      },
    });
  }

  intervalId: any;

  loadmore(): void {
    this.loading = true;
    this.api.dashboard
      .get(DEFAULT_NUM_ITEMS, this.items[this.items.length - 1].job.id)
      .subscribe({
        next: (items) => {
          this.items.push(...items);
          this.loading = false;
          if (items.length < DEFAULT_NUM_ITEMS) this.hasMore = false;
        },
        error: (e) => {
          if (e instanceof HttpErrorResponse) {
            this.errorMessage = e.message;
          } else {
            this.errorMessage = JSON.stringify(e);
          }
          console.warn(e);
          this.error = true;
          this.loading = false;
          this.hasMore = false;
        },
      });
  }

  ngOnInit(): void {
    this.title.setTitle('Rurikawa', 'dashboard');
    this.error = false;
    this.errorMessage = undefined;
    this.api.dashboard.get(DEFAULT_NUM_ITEMS).subscribe({
      next: (items) => {
        this.items = items;
        this.loading = false;
        if (items.length < DEFAULT_NUM_ITEMS) this.hasMore = false;
      },
      error: (e) => {
        if (e instanceof HttpErrorResponse) {
          this.errorMessage = e.message;
        } else {
          this.errorMessage = JSON.stringify(e);
        }
        console.warn(e);
        this.error = true;
        this.loading = false;
      },
    });
    this.judgerStatusService.subscribeData(
      (status) => (this.judgerStat = status)
    );
    this.judgerStatusService.subscribeQueueData(
      (queueStatus) => (this.queueStatus = queueStatus)
    );
    this.judgerStatusService
      .getAssembly()
      .then((asm) => (this.assemblyInfo = asm));
    this.fetchAnnouncements();

    this.judgerStatusService.triggerUpdate().then();
    this.intervalId = setInterval(
      () => this.judgerStatusService.triggerUpdate().then(),
      10000
    );
  }

  ngOnDestroy() {
    this.title.revertTitle();
    clearInterval(this.intervalId);
  }
}
