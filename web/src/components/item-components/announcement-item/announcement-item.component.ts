import { Component, Input, OnInit } from '@angular/core';
import { Announcement } from 'src/models/server-types';
import marked from 'marked';
import dayjs from 'dayjs';

@Component({
  selector: 'app-announcement-item',
  templateUrl: './announcement-item.component.html',
  styleUrls: ['./announcement-item.component.styl'],
})
export class AnnouncementItemComponent implements OnInit {
  constructor() {}

  private firstPara: string;

  bodyBrief: string;

  @Input() item: Announcement;

  get sendTime() {
    return dayjs(this.item.sendTime).local().format('YYYY-MM-DD');
  }

  ngOnInit(): void {
    let tokens = marked.lexer(this.item.body);
    try {
      marked.walkTokens(tokens, (token: any) => {
        if (token.type === 'paragraph') {
          throw new Error(token.raw);
        }
      });
    } catch (e) {
      if (e instanceof Error) {
        this.bodyBrief = e.message;
        return;
      }
    }
    this.bodyBrief = '_这个公告没有文字内容_';
  }
}
