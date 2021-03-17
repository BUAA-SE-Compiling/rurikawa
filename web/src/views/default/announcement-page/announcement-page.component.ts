import { HttpErrorResponse } from '@angular/common/http';
import { Component, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import dayjs from 'dayjs';
import { Announcement } from 'src/models/server-types';
import { AccountService } from 'src/services/account_service';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-announcement-page',
  templateUrl: './announcement-page.component.html',
  styleUrls: ['./announcement-page.component.styl'],
})
export class AnnouncementPageComponent implements OnInit {
  constructor(
    private api: ApiService,
    private route: ActivatedRoute,
    private router: Router,
    private account:AccountService
  ) {}

  id: string;
  loading = true;
  announcement?: Announcement;
  error?: HttpErrorResponse;

  get announcementTime() {
    if (this.announcement === undefined) return;
    return dayjs(this.announcement.sendTime).format('YYYY-MM-DD hh:mm');
  }

  get isAdmin(){return this.account.isAdmin}

  ngOnInit(): void {
    this.route.paramMap.subscribe((map) => {
      const id = map.get('id');
      this.id = id;
      this.api.announcement.get(id).subscribe({
        next: (a) => {
          this.announcement = a;
          this.loading = false;
        },
        error: (e) => {
          if (e instanceof HttpErrorResponse) {
            if (e.status === 404) {
              this.router.navigate(['/404']);
            }
          }
        },
      });
    });
  }
}
