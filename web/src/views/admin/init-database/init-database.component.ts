import { Component, OnInit } from '@angular/core';

@Component({
  selector: 'app-init-database',
  templateUrl: './init-database.component.html',
  styleUrls: ['./init-database.component.styl'],
})
export class InitDatabaseComponent implements OnInit {
  constructor() {}

  username: string = '';
  password: string = '';
  message: string | undefined;

  warnUsername: boolean = false;
  warnPassword: boolean = false;

  ngOnInit(): void {}
}
