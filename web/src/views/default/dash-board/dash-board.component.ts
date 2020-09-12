import { Component, OnInit } from '@angular/core';
import { SliderItem } from 'src/components/base-components/slider-view/slider-view.component';
import { DashboardItem } from 'src/models/job-items';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { environment } from 'src/environments/environment';
import { endpoints } from 'src/environments/endpoints';

@Component({
  selector: 'app-dash-board',
  templateUrl: './dash-board.component.html',
  styleUrls: ['./dash-board.component.styl'],
})
export class DashBoardComponent implements OnInit {
  constructor(private router: Router, private httpClient: HttpClient) {}
  loading = true;
  items: DashboardItem[] | undefined = undefined;

  gotoJudgeSuite(id: string) {
    this.router.navigate(['/suite', id]);
  }

  ngOnInit(): void {
    this.httpClient
      .get<DashboardItem[]>(environment.endpointBase + endpoints.dashboard.get)
      .subscribe({
        next: (items) => {
          this.items = items;
          this.loading = false;
        },
        error: (e) => {
          console.warn(e);
          this.loading = false;
        },
      });
  }
}

function getSampleItems(): DashboardItem[] {
  return [
    {
      id: '123j2dp',
      name: '编译大作业1',
      totalItem: 8,
      finishedItem: 6,
      endTime: new Date('2020-11-30T12:34:56Z'),
      status: [
        { status: 'Accepted', cnt: 16 },
        { status: 'WrongAnswer', cnt: 5 },
        { status: 'Running', cnt: 3 },
        { status: 'NotRunned', cnt: 4 },
      ],
    },
    {
      id: '123j2dq',
      name: '编译大作业0',
      totalItem: 8,
      finishedItem: 6,
      endTime: new Date('2020-09-06T12:34:56Z'),
      status: [{ status: 'Accepted', cnt: 50 }],
    },
  ];
}
