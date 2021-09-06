import { Component, OnInit, OnDestroy } from '@angular/core';
import GitHubIcon from '@iconify/icons-carbon/logo-github';
import { TitleService } from 'src/services/title_service';

@Component({
  selector: 'app-about-page',
  templateUrl: './about-page.component.html',
  styleUrls: ['./about-page.component.less'],
})
export class AboutPageComponent implements OnInit, OnDestroy {
  constructor(private title: TitleService) {}

  githubIcon = GitHubIcon;

  ngOnInit(): void {
    this.title.setTitle('About');
  }

  ngOnDestroy(): void {
    this.title.revertTitle();
  }
}
