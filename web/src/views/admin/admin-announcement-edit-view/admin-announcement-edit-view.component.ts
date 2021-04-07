import { Location } from '@angular/common';
import { Component, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import {} from '@ng-util/markdown/public-api';
import { Announcement } from 'src/models/server-types';
import { AccountService } from 'src/services/account_service';
import { ApiService } from 'src/services/api_service';

@Component({
  selector: 'app-admin-announcement-edit-view',
  templateUrl: './admin-announcement-edit-view.component.html',
  styleUrls: ['./admin-announcement-edit-view.component.styl'],
})
export class AdminAnnouncementEditViewComponent implements OnInit {
  constructor(
    private route: ActivatedRoute,
    private location: Location,
    private router: Router,
    private account: AccountService,
    private api: ApiService
  ) {
    route.data.subscribe((data) => {
      this.editingNew = data.new;
    });
    route.paramMap.subscribe((map) => {
      if (map.has('id')) {
        this.announcement.id = map.get('id');
        // this.fetchAnnouncement();
      } else this.announcement.id = '';
    });
  }

  editingNew: boolean;

  get nuOptions() {
    return {
      minHeight: 400,
      theme: 'custom',
      icon: 'material',
      placeholder: '填入内容',
      mode: 'ir',
      value: this.announcement.body,
      toolbar: [
        'bold',
        'italic',
        'strike',
        'quote',
        'line',
        '|',
        'headings',
        'list',
        'ordered-list',
        '|',
        'code',
        'inline-code',
        'link',
        'table',
      ],
      preview: {
        mode: 'editor',
        hljs: {
          style: 'dracula',
        },
        theme: { current: null },
      },
    };
  }

  announcement: Announcement = {
    id: '',
    title: '',
    body: '',
    sender: this.account.Username,
    sendTime: new Date(),
    kind: 'Generic',
    tags: [],
  };

  saveAnnouncement() {
    if (this.editingNew) {
      this.api.announcement.post(this.announcement).subscribe((id) => {
        // this.location.replaceState(`/admin/announcement/edit/${id}`);
        this.announcement.id = id;
        this.editingNew = false;
        this.router.navigate(['/announcement', id]);
      });
    } else {
      this.api.announcement
        .set(this.announcement.id, this.announcement)
        .subscribe(() => {
          this.router.navigate(['/announcement', this.announcement.id]);
        });
    }
  }

  fetchAnnouncement() {
    if (this.announcement?.id) {
      this.api.announcement.get(this.announcement.id).subscribe((a) => {
        this.announcement = a;
      });
    }
  }

  deleteAnnouncement() {
    if (this.announcement?.id) {
      this.api.announcement.delete(this.announcement.id).subscribe((a) => {
        this.location.back();
      });
    }
  }

  ngOnInit(): void {}
}
