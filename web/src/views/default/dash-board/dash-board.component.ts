import { Component, OnInit, OnDestroy } from '@angular/core';
import { SliderItem } from 'src/components/base-components/slider-view/slider-view.component';
// import { DashboardItem } from 'src/models/job-items';
import { Router } from '@angular/router';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';
import { DashboardItem } from 'src/models/server-types';
import { TitleService } from 'src/services/title_service';

@Component({
  selector: 'app-dash-board',
  templateUrl: './dash-board.component.html',
  styleUrls: ['./dash-board.component.styl'],
})
export class DashBoardComponent implements OnInit, OnDestroy {
  constructor(
    private router: Router,
    private httpClient: HttpClient,
    private title: TitleService
  ) {}
  loading = true;
  items: DashboardItem[] | undefined = undefined;

  error: boolean = false;
  errorMessage?: string;

  gotoJudgeSuite(id: string) {
    this.router.navigate(['/suite', id]);
  }

  ngOnInit(): void {
    this.title.setTitle('Rurikawa', 'dashboard');
    this.error = false;
    this.errorMessage = undefined;
    this.httpClient
      .get<DashboardItem[]>(
        environment.endpointBase() + endpoints.dashboard.get
      )
      .subscribe({
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
  }

  ngOnDestroy() {
    this.title.revertTitle();
  }
}
