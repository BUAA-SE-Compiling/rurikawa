import { HttpClient } from '@angular/common/http';
import { Component, OnInit } from '@angular/core';
import { endpoints } from 'src/environments/endpoints';
import { environment } from 'src/environments/environment';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-admin-manage-judger-view',
  templateUrl: './admin-manage-judger-view.component.html',
  styleUrls: ['./admin-manage-judger-view.component.styl'],
})
export class AdminManageJudgerViewComponent implements OnInit {
  constructor(private httpClient: HttpClient, private api: ApiService) {}

  tokenRequested?: string;

  ngOnInit(): void {}

  requestToken() {
    this.api.admin.getJudgerRegisterToken(false, []).subscribe({
      next: (val) => {
        this.tokenRequested = val;
      },
    });
  }
}
