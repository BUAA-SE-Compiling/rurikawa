import { Component, OnInit, OnDestroy } from '@angular/core';
import GitHubIcon from '@iconify/icons-carbon/logo-github';
import {
  NavbarColorScheme,
  NavbarDisplayKind,
  NavbarService,
} from 'src/services/navbar_service';
import { TitleService } from 'src/services/title_service';

@Component({
  selector: 'app-about-page',
  templateUrl: './about-page.component.html',
  styleUrls: ['./about-page.component.less'],
})
export class AboutPageComponent implements OnInit, OnDestroy {
  constructor(
    private title: TitleService,
    private navbarService: NavbarService
  ) {
    navbarService.pushStyle(
      {
        color: NavbarColorScheme.Accent,
        display: NavbarDisplayKind.Collapse,
        is_admin_mode: false,
      },
      true
    );
  }

  githubIcon = GitHubIcon;

  ngOnInit(): void {
    this.title.setTitle('About');
  }

  ngOnDestroy(): void {
    this.title.revertTitle();
  }
}
