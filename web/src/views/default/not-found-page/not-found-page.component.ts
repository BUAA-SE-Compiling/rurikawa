import { Component, OnInit } from '@angular/core';
import { Location } from '@angular/common';

@Component({
  selector: 'app-not-found-page',
  templateUrl: './not-found-page.component.html',
  styleUrls: ['./not-found-page.component.styl'],
})
export class NotFoundPageComponent implements OnInit {
  constructor(private router: Location) {}

  ngOnInit(): void {}

  back() {
    this.router.back();
  }
}
