import { Component, OnInit, Input } from '@angular/core';
import { AccountService } from 'src/services/account_service';

@Component({
  selector: 'app-navbar',
  templateUrl: './navbar.component.html',
  styleUrls: ['./navbar.component.styl'],
})
export class NavbarComponent implements OnInit {
  constructor(public accountService: AccountService) {}

  @Input() adminMode: boolean = false;
  @Input() subdir: string | undefined = undefined;

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
