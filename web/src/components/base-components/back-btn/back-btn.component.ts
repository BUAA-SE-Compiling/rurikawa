import { Component, OnInit } from '@angular/core';
import { Location } from '@angular/common';
import LeftIcon from '@iconify/icons-carbon/arrow-left';

@Component({
  selector: 'app-back-btn',
  templateUrl: './back-btn.component.html',
  styleUrls: ['./back-btn.component.less'],
})
export class BackBtnComponent {
  constructor(private location: Location) {}

  readonly leftIcon = LeftIcon;

  back() {
    this.location.back();
  }
}
