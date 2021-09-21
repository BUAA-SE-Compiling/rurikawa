import { Component, EventEmitter, Input, OnInit, Output } from '@angular/core';
import { AccountAndProfile } from 'src/models/server-types';

@Component({
  selector: 'app-user-item',
  templateUrl: './user-item.component.html',
  styleUrls: ['./user-item.component.less'],
})
export class UserItemComponent implements OnInit {
  constructor() {}

  @Input() user: AccountAndProfile;
  @Output() passwordChange = new EventEmitter<string>();

  collapsed = true;

  toggleCollapsed() {
    this.collapsed = !this.collapsed;
  }

  newPassword = '';
  emitPasswordChange() {
    this.passwordChange.emit(this.newPassword);
  }

  ngOnInit(): void {}
}
