import { HttpErrorResponse } from '@angular/common/http';
import { Component, OnInit } from '@angular/core';
import { TextboxComponent } from 'src/components/base-components/textbox/textbox.component';
import { UserKind } from 'src/models/server-types';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-admin-add-user-view',
  templateUrl: './admin-add-user-view.component.html',
  styleUrls: ['./admin-add-user-view.component.less'],
})
export class AdminAddUserViewComponent implements OnInit {
  constructor(private api: ApiService) {}

  options = ['User', 'Admin', 'Root'];

  username: string;
  password: string;
  kind: string;

  result: string;
  sending: boolean = false;

  send() {
    console.log(this);
    if (!this.username) {
      this.result = '请填写用户名';
      return;
    } else if (!this.password) {
      this.result = '请填写密码';
      return;
    } else if (!this.kind) {
      this.result = '请填写用户类型';
      return;
    }

    this.sending = true;
    let kind = this.kind as UserKind;
    this.api.admin.registerUser(this.username, this.password, kind).subscribe({
      next: (res) => {
        this.result = '创建成功';
      },
      error: (e) => {
        if (e instanceof HttpErrorResponse) {
          this.result = e.message;
        }
      },
      complete: () => (this.sending = false),
    });
  }

  ngOnInit(): void {}
}
