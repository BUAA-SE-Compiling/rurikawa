import { Component, OnInit } from '@angular/core';
import { SliderItem } from 'src/components/base-components/slider-view/slider-view.component';
import { DashboardItem } from 'src/models/job-items';

@Component({
  selector: 'app-dash-board',
  templateUrl: './dash-board.component.html',
  styleUrls: ['./dash-board.component.styl'],
})
export class DashBoardComponent implements OnInit {
  constructor() {}

  items: DashboardItem[] = [
    {
      id: '123j2dp',
      name: '编译大作业1',
      totalItem: 8,
      finishedItem: 6,
      endTime: new Date('2020-11-30T12:34:56Z'),
      status: [
        { status: 'ac', cnt: 16 },
        { status: 'wa', cnt: 5 },
        { status: 'tle', cnt: 3 },
        { status: 'ce', cnt: 4 },
      ],
    },
    {
      id: '123j2dq',
      name: '编译大作业0',
      totalItem: 8,
      finishedItem: 6,
      endTime: new Date('2020-09-06T12:34:56Z'),
      status: [{ status: 'ac', cnt: 50 }],
    },
  ];

  ngOnInit(): void {}
}
