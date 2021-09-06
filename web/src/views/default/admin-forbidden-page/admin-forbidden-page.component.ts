import { Component, OnInit } from '@angular/core';
import { Location } from '@angular/common';

@Component({
  selector: 'app-forbidden-page',
  templateUrl: './admin-forbidden-page.component.html',
  styleUrls: ['./admin-forbidden-page.component.less'],
})
export class AdminForbiddenPageComponent implements OnInit {
  constructor(private router: Location) {}

  ngOnInit(): void {}

  back() {
    this.router.back();
  }
}
