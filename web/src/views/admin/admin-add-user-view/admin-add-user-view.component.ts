import { Component, OnInit } from '@angular/core';
import { TextboxComponent } from 'src/components/base-components/textbox/textbox.component';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-admin-add-user-view',
  templateUrl: './admin-add-user-view.component.html',
  styleUrls: ['./admin-add-user-view.component.styl'],
})
export class AdminAddUserViewComponent implements OnInit {
  constructor(private api: ApiService) {}

  username: string;
  password: string;
  kind: string;

  send() {
    // this.api.profile.
  }

  ngOnInit(): void {}
}
