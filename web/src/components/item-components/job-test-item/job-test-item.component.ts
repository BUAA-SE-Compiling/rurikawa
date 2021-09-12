import { Component, OnInit, Input } from '@angular/core';
import {
  TestResult,
  dashboardTypeToSlider,
  sliderKindToCssVariable,
} from 'src/models/job-items';

@Component({
  selector: 'app-job-test-item',
  templateUrl: './job-test-item.component.html',
  styleUrls: ['./job-test-item.component.less'],
})
export class JobTestItemComponent implements OnInit {
  constructor() {}

  @Input() key: string;
  @Input() item: TestResult;
  @Input() baseScore?: number;
  get score(): number | undefined {
    return this.item.score;
  }

  displayResult() {
    switch (this.item.kind) {
      case 'Accepted':
        return 'AC';
      case 'MemoryLimitExceeded':
        return 'MLE';
      case 'NotRan':
        return 'NR';
      case 'OtherError':
        return 'OE';
      case 'PipelineFailed':
        return 'PF';
      case 'Running':
        return 'RUN';
      case 'RuntimeError':
        return 'RE';
      case 'TimeLimitExceeded':
        return 'TLE';
      case 'Waiting':
        return 'QU';
      case 'WrongAnswer':
        return 'WA';
      case 'ShouldFail':
        return 'SFE';
    }
  }

  backgroundColor() {
    return (
      'var(' +
      sliderKindToCssVariable(dashboardTypeToSlider(this.item.kind)) +
      ')'
    );
  }

  ngOnInit(): void {}
}
