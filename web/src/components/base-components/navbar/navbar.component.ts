import { Component, OnInit, Input, Injectable } from '@angular/core';
import { AccountService } from 'src/services/account_service';
import {
  trigger,
  state,
  style,
  transition,
  animate,
} from '@angular/animations';
import { ActivatedRoute, NavigationEnd, Router } from '@angular/router';
import {
  NavbarService,
  NavbarColorScheme,
  NavbarDisplayKind,
} from 'src/services/navbar_service';

@Component({
  selector: 'app-navbar',
  templateUrl: './navbar.component.html',
  styleUrls: ['./navbar.component.styl'],
  animations: [
    trigger('adminBar', [
      transition(':enter', [
        style({
          height: '0px',
        }),
        animate('100ms', style({ height: '8px' })),
      ]),
      transition(':leave', animate('100ms', style({ height: '0px' }))),
    ]),
  ],
})
export class NavbarComponent implements OnInit {
  constructor(
    public accountService: AccountService,
    private svc: NavbarService,
    private router: Router,
    private route: ActivatedRoute
  ) {
    this.adminMode = false;
    this.router.events.subscribe((ev) => {
      console.log(ev);
      if (ev instanceof NavigationEnd) {
        if (ev.url.startsWith('/admin')) {
          this.adminMode = true;
        } else {
          this.adminMode = false;
        }
      }
    });
  }

  adminMode: boolean;

  @Input() subdir: string | undefined = undefined;
  @Input() hideLogo: boolean = false;
  @Input() logoLink: string = '/';

  get realSubir() {
    return this.subdir ?? this.adminMode ? 'admin' : undefined;
  }

  get style() {
    return {
      'admin-mode': this.adminMode,
      'style-accent': this.svc.currentStyle.color == NavbarColorScheme.Accent,
      hide: this.svc.currentStyle.display == NavbarDisplayKind.None,
      'collapse-space':
        this.svc.currentStyle.display == NavbarDisplayKind.Collapse,
    };
  }

  get username() {
    return this.accountService.Username;
  }
  logout() {
    this.accountService.logout();
    this.router.navigate(['/']);
  }

  ngOnInit(): void {}
}
