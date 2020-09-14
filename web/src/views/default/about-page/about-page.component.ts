import { Component, OnInit } from '@angular/core';
import GitHubIcon from '@iconify/icons-carbon/logo-github';

@Component({
  selector: 'app-about-page',
  templateUrl: './about-page.component.html',
  styleUrls: ['./about-page.component.styl'],
})
export class AboutPageComponent implements OnInit {
  constructor() {}

  githubIcon = GitHubIcon;

  ngOnInit(): void {}
}
