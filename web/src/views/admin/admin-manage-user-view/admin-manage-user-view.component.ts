import { HttpErrorResponse } from '@angular/common/http';
import { Component, OnInit } from '@angular/core';
import { AccountAndProfile } from 'src/models/server-types';
import { ApiService } from 'src/services/api_service';

const ItemCountPerPage = 50;
@Component({
  selector: 'app-admin-manage-user-view',
  templateUrl: './admin-manage-user-view.component.html',
  styleUrls: ['./admin-manage-user-view.component.less'],
})
export class AdminManageUserViewComponent implements OnInit {
  constructor(private apiService: ApiService) {}

  searchParams = {
    username: '',
    studentId: '',
    userType: '',
  };

  errorResult = '';
  searchResult: AccountAndProfile[] = [];
  performedSearch = false;
  loading = false;
  maxUsername = '';
  searchExhausted = false;

  search() {
    this.performedSearch = true;
    this.loading = true;
    this.apiService.admin
      .searchUserInfo(
        this.getSearchParams(),
        this.maxUsername,
        false,
        ItemCountPerPage
      )
      .subscribe({
        next: (v) => {
          this.searchResult = v;
          this.searchExhausted = v.length < ItemCountPerPage;
        },
        error: (e) => {
          if (e instanceof HttpErrorResponse) {
            this.errorResult = e.message;
          }
        },
        complete: () => {
          this.loading = false;
        },
      });
  }

  private getSearchParams(): {
    usernameLike?: string;
    kind?: string;
    studentId?: string;
    searchNameUsingRegex: boolean;
  } {
    return {
      usernameLike: this.searchParams.username
        ? this.searchParams.username
        : undefined,
      studentId: this.searchParams.studentId
        ? this.searchParams.studentId
        : undefined,
      kind: this.searchParams.userType ? this.searchParams.userType : undefined,
      searchNameUsingRegex: true,
    };
  }

  loadMore() {
    this.performedSearch = true;
    this.loading = true;
    this.apiService.admin
      .searchUserInfo(
        this.getSearchParams(),
        this.maxUsername,
        false,
        ItemCountPerPage
      )
      .subscribe({
        next: (v) => {
          this.searchResult.push(...v);
          this.searchExhausted = v.length < ItemCountPerPage;
        },
        error: (e) => {
          if (e instanceof HttpErrorResponse) {
            this.errorResult = e.message;
          }
        },
        complete: () => {
          this.loading = false;
        },
      });
  }

  changePassword(username: string, password: string) {
    this.apiService.admin.editPassword(username, password).subscribe();
  }

  ngOnInit(): void {}
}
