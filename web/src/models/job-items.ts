import { SliderItemKind } from 'src/components/base-components/slider-view/slider-view.component';

export function dashboardTypeToSlider(item: TestResultKind): SliderItemKind {
  switch (item) {
    case 'Accepted':
      return 'accept';
    case 'WrongAnswer':
      return 'error';
    case 'MemoryLimitExceeded':
      return 'warn';
    case 'NotRunned':
      return 'cancel';
    case 'OtherError':
      return 'disable';
    case 'TimeLimitExceeded':
      return 'warn';
    case 'PipelineFailed':
      return 'warn';
    case 'Running':
      return 'info';
    case 'RuntimeError':
      return 'warn';
    case 'Waiting':
      return 'disable';
  }
}

export function sliderKindToCssVariable(kind: SliderItemKind): string {
  switch (kind) {
    case 'accept':
      return '--success-color';
    case 'error':
      return '--error-color';
    case 'warn':
      return '--warning-color';
    case 'info':
      return '--info-color';
    case 'info-alt':
      return '--info-alt-color';
    case 'disable':
      return '--disabled-color';
    case 'cancel':
      return '--gray-color';
  }
}

export interface DashboardItem {
  id: string;
  name: string;
  endTime: Date;
  finishedItem: number;
  totalItem: number;
  status: { status: TestResultKind; cnt: number }[];
}

export interface JobItem {
  id: string;
  time: Date;
  finishedItem: number;
  totalItem: number;
  status: { status: TestResultKind; cnt: number }[];
}

export type JobStage =
  | 'Queued'
  | 'Dispatched'
  | 'Fetching'
  | 'Compiling'
  | 'Running'
  | 'Finished'
  | 'Cancelled';

export type JobResultKind =
  | 'Accepted'
  | 'CompileError'
  | 'PipelineError'
  | 'JudgerError'
  | 'Aborted'
  | 'OtherError';

export type TestResultKind =
  | 'Accepted'
  | 'WrongAnswer'
  | 'RuntimeError'
  | 'PipelineFailed'
  | 'TimeLimitExceeded'
  | 'MemoryLimitExceeded'
  | 'NotRunned'
  | 'Waiting'
  | 'Running'
  | 'OtherError';

export interface TestResult {
  kind: TestResultKind;
  resultFileId: string | undefined;
}

export interface Job {
  id: string;
  account: string;
  repo: string;
  branch: string | undefined;
  testSuite: string;
  tests: string[];
  stage: JobStage;
  resultKind: JobResultKind;
  resultMessage: string | undefined;
  results: { [key: string]: TestResult };
}
