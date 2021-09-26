import { SliderItemKind } from 'src/components/base-components/slider-view/slider-view.component';
import { mapValues, groupBy, toPairs } from 'lodash';
import { extractTime } from './flowsnake';
import { Dayjs } from 'dayjs';
import { TestSuite } from './server-types';
import { resultBriefMain, resultBriefSub } from 'src/util/brief-calc';

export function dashboardTypeToSlider(item: TestResultKind): SliderItemKind {
  switch (item) {
    case 'Accepted':
      return 'accept';
    case 'WrongAnswer':
      return 'error';
    case 'MemoryLimitExceeded':
      return 'warn';
    case 'NotRan':
      return 'cancel';
    case 'OtherError':
      return 'info-alt';
    case 'TimeLimitExceeded':
      return 'warn';
    case 'PipelineFailed':
      return 'warn';
    case 'Running':
      return 'info';
    case 'RuntimeError':
      return 'warn';
    case 'ShouldFail':
      return 'error';
    case 'Waiting':
      return 'disable';
    default:
      return item;
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

interface JobStatus {
  status: TestResultKind;
  cnt: number;
}

// export interface DashboardItem {
//   id: string;
//   name: string;
//   endTime: Date;
//   finishedItem: number;
//   totalItem: number;
//   status: { status: TestResultKind; cnt: number }[];
// }

export interface JobItem {
  id: string;
  account: string;
  time: Dayjs;
  numberBrief: string;
  numberBriefSub?: string;
  status: JobStatus[];
  repo: string;
  branch: string;
  revision: string;
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
  | 'ShouldFail'
  | 'NotRan'
  | 'Waiting'
  | 'Running'
  | 'OtherError';

export interface TestResult {
  kind: TestResultKind;
  score?: number;
  resultFileId: string | undefined;
}

export interface Job {
  id: string;
  account: string;
  repo: string;
  branch?: string;
  revision: string;
  testSuite: string;
  tests: string[];
  stage: JobStage;
  resultKind: JobResultKind;
  resultMessage?: string;
  buildOutputFile?: string;
  results: { [key: string]: TestResult };
}

export interface ProcessInfo {
  ret_code: number;
  command: string;
  stdout: string;
  stderr: string;
}

export interface FailedTestcaseOutput {
  output: ProcessInfo[];
  stdoutDiff?: string;
  message?: string;
}

export interface Diff {
  kind: ' ' | '-' | '+';
  line: string;
}

export function unDiff(input: string): Diff[] {
  let lines = input.split('\n');
  let arr = [];
  for (let line of lines) {
    if (line === '') {
      continue;
    }
    let head = line[0];
    let rest = line.substr(2);

    arr.push({ kind: head, line: rest });
  }
  return arr;
}

export function getStatus(job: Job): JobStatus[] {
  if (job === undefined) {
    return [{ status: 'Waiting', cnt: 1 }];
    // } else if (job.stage !== 'Finished') {
    //   return [{ status: 'Waiting', cnt: 1 }];
  } else if (job.stage !== 'Finished') {
    return [{ status: 'Waiting', cnt: 1 }];
  } else if (job.resultKind !== 'Accepted') {
    return [{ status: 'OtherError', cnt: 1 }];
  }
  let res = toPairs(
    mapValues(
      groupBy(job.results, (result) => dashboardTypeToSlider(result.kind)),
      (i) => i.length
    )
  ).map(([x, y]) => {
    return { status: x as TestResultKind, cnt: y };
  });
  if (res.length === 0) {
    return [{ status: 'Waiting', cnt: job.tests.length }];
  } else {
    return res;
  }
}

const numberFormatter = Intl.NumberFormat('native', {
  maximumSignificantDigits: 5,
});

export function JobToJobItem(job: Job, testSuite?: TestSuite): JobItem {
  let res = {
    id: job.id,
    account: job.account,
    numberBrief: resultBriefMain(job, testSuite, numberFormatter),
    numberBriefSub: resultBriefSub(job, testSuite, numberFormatter),
    status: getStatus(job),
    time: extractTime(job.id),
    repo: job.repo,
    branch: job.branch ?? 'HEAD',
    revision: job.revision,
  };
  return res;
}
