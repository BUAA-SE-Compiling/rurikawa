import { Component, OnInit } from '@angular/core';
import { AccountService } from 'src/services/account_service';

@Component({
  selector: 'app-navbar',
  templateUrl: './navbar.component.html',
  styleUrls: ['./navbar.component.styl'],
})
export class NavbarComponent implements OnInit {
  constructor(public accountService: AccountService) {}

  ngOnInit(): void {}
}
