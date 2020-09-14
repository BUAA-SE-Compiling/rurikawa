import { Component, OnInit, Input } from '@angular/core';
import { AccountService } from 'src/services/account_service';
import {
  trigger,
  state,
  style,
  transition,
  animate,
} from '@angular/animations';

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
  constructor(public accountService: AccountService) {}

  @Input() adminMode: boolean = false;
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
  }

  ngOnInit(): void {}
}
