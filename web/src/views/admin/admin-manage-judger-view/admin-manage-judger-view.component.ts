import { HttpClient } from '@angular/common/http';
import { Component, ElementRef, OnInit, ViewChild } from '@angular/core';
import { endpoints } from 'src/environments/endpoints';
import { environment } from 'src/environments/environment';
import { ApiService } from 'src/services/api_service';
import iconCalendar from '@iconify/icons-carbon/calendar';
import iconTime from '@iconify/icons-carbon/time';
import iconTag from '@iconify/icons-carbon/tag';

interface TokenParams {
  isSingleUse: boolean;
  expires: boolean;
  expiresAtDate: string;
  expiresAtTime: string;
  tags: string;
}

@Component({
  selector: 'app-admin-manage-judger-view',
  templateUrl: './admin-manage-judger-view.component.html',
  styleUrls: ['./admin-manage-judger-view.component.less'],
})
export class AdminManageJudgerViewComponent implements OnInit {
  constructor(private httpClient: HttpClient, private api: ApiService) {}

  readonly iconCalendar = iconCalendar;
  readonly iconTime = iconTime;
  readonly iconTag = iconTag;

  tokenRequested: string = '';

  tokenParams: TokenParams = {
    isSingleUse: false,
    expires: false,
    expiresAtDate: '',
    expiresAtTime: '',
    tags: '',
  };

  @ViewChild('token') tokenInput: ElementRef<HTMLInputElement>;

  ngOnInit(): void {}

  requestToken() {
    this.api.admin
      .getJudgerRegisterToken(
        this.tokenParams.expires,
        this.tokenParams.tags.split(',').map((x) => x.trim()),
        new Date(
          this.tokenParams.expiresAtDate + 'T' + this.tokenParams.expiresAtTime
        )
      )
      .subscribe({
        next: (val) => {
          this.tokenRequested = val;
        },
      });
  }

  selectToken() {
    this.tokenInput.nativeElement.select();
  }

  minExpireTime() {
    // return new Date()
  }
}
