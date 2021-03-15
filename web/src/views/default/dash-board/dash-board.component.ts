import { Component, OnInit, OnDestroy } from '@angular/core';
// import { DashboardItem } from 'src/models/job-items';
import { Router } from '@angular/router';
import { HttpErrorResponse } from '@angular/common/http';
import { DashboardItem, JudgerStatus } from 'src/models/server-types';
import { TitleService } from 'src/services/title_service';
import { JudgerStatusService } from 'src/services/judger_status_service';
import { Subscription } from 'rxjs';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-dash-board',
  templateUrl: './dash-board.component.html',
  styleUrls: ['./dash-board.component.styl'],
})
export class DashBoardComponent implements OnInit, OnDestroy {
  constructor(
    private router: Router,
    private api: ApiService,
    private title: TitleService,
    public judgerStatusService: JudgerStatusService
  ) {}
  loading = true;
  items: DashboardItem[] | undefined = undefined;

  error: boolean = false;
  errorMessage?: string;

  judgerStat?: JudgerStatus;
  judgerSubscription?: Subscription;

  gotoJudgeSuite(id: string) {
    this.router.navigate(['/suite', id]);
  }

  fetchJudgerStat() {
    this.judgerStatusService.getData().then((v) => {
      this.judgerStat = v;
      console.log(this.judgerStat);
    });
  }

  ngOnInit(): void {
    this.title.setTitle('Rurikawa', 'dashboard');
    this.error = false;
    this.errorMessage = undefined;
    this.api.dashboard.get().subscribe({
      next: (items) => {
        this.items = items;
        this.loading = false;
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
    this.fetchJudgerStat();
  }

  ngOnDestroy() {
    this.title.revertTitle();
    this.judgerSubscription?.unsubscribe();
  }
}
