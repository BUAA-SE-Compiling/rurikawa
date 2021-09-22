import { Component, OnInit } from '@angular/core';
import { Location } from '@angular/common';
import {
  NavbarColorScheme,
  NavbarDisplayKind,
  NavbarService,
} from 'src/services/navbar_service';

@Component({
  selector: 'app-not-found-page',
  templateUrl: './not-found-page.component.html',
  styleUrls: ['./not-found-page.component.less'],
})
export class NotFoundPageComponent implements OnInit {
  constructor(private router: Location, private navbarService: NavbarService) {
    this.navbarService.pushStyle({
      color: NavbarColorScheme.Accent,
      display: NavbarDisplayKind.Normal,
      is_admin_mode: false,
    });
  }

  ngOnInit(): void {}

  back() {
    this.router.back();
  }
}
