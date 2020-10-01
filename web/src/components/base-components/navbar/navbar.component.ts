import { Component, OnInit, Input, Injectable } from '@angular/core';
import { AccountService } from 'src/services/account_service';
import {
  trigger,
  state,
  style,
  transition,
  animate,
} from '@angular/animations';
import { Router } from '@angular/router';

@Injectable({ providedIn: 'root' })
export class NavbarHelper {
  public isAdminMode: boolean = false;
}

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
    private navbarHelper: NavbarHelper,
    private router: Router
  ) {
    this.adminMode = false;
  }

  @Input() set adminMode(val: boolean) {
    this.navbarHelper.isAdminMode = val;
  }
  get adminMode() {
    return this.navbarHelper.isAdminMode;
  }

  @Input() subdir: string | undefined = undefined;
  @Input() hideLogo: boolean = false;
  @Input() logoLink: string = '/';

  get realSubir() {
    return this.subdir ?? this.adminMode ? 'admin' : undefined;
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
