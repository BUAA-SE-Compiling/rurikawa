import { HttpClient } from '@angular/common/http';
import { Component, OnInit } from '@angular/core';
import { endpoints } from 'src/environments/endpoints';
import { environment } from 'src/environments/environment';

@Component({
  selector: 'app-admin-manage-judger-view',
  templateUrl: './admin-manage-judger-view.component.html',
  styleUrls: ['./admin-manage-judger-view.component.styl'],
})
export class AdminManageJudgerViewComponent implements OnInit {
  constructor(private httpClient: HttpClient) {}

  tokenRequested?: string;

  ngOnInit(): void {}

  requestToken() {
    this.httpClient
      .post(
        environment.endpointBase() + endpoints.admin.judgerRegisterToken,
        {
          isSingleUse: false,
          tags: [],
        },
        { responseType: 'text' }
      )
      .subscribe({
        next: (val) => {
          this.tokenRequested = val;
        },
      });
  }
}
