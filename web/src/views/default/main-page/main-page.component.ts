import { Component, OnInit } from '@angular/core';
import {
  NavbarColorScheme,
  NavbarDisplayKind,
  NavbarService,
} from 'src/services/navbar_service';

@Component({
  selector: 'app-main-page',
  templateUrl: './main-page.component.html',
  styleUrls: ['./main-page.component.less'],
})
export class MainPageComponent implements OnInit {
  constructor(private navbarService: NavbarService) {
    navbarService.pushStyle({
      color: NavbarColorScheme.Accent,
      is_admin_mode: false,
      display: NavbarDisplayKind.Normal,
    });
  }

  ngOnInit(): void {}
}
