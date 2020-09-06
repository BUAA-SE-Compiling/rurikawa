import { Component, OnInit } from '@angular/core';
import { Route } from '@angular/compiler/src/core';
import { Router } from '@angular/router';

@Component({
  selector: 'app-test-suite-view',
  templateUrl: './test-suite-view.component.html',
  styleUrls: ['./test-suite-view.component.styl'],
})
export class TestSuiteViewComponent implements OnInit {
  constructor(private route: Router) {}

  ngOnInit(): void {}
}
