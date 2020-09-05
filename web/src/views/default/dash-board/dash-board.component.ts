import { Component, OnInit } from '@angular/core';
import { SliderItem } from 'src/components/base-components/slider-view/slider-view.component';

@Component({
  selector: 'app-dash-board',
  templateUrl: './dash-board.component.html',
  styleUrls: ['./dash-board.component.styl'],
})
export class DashBoardComponent implements OnInit {
  constructor() {}

  items: SliderItem[] = [
    {
      kind: 'accept',
      num: 5,
    },
    {
      kind: 'warn',
      num: 3,
    },
    {
      kind: 'info-alt',
      num: 1,
    },
    {
      kind: 'disable',
      num: 2,
    },
  ];
  ngOnInit(): void {}
}
