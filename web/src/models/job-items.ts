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
  ret_code: ExitStatus;
  command: string;
  stdout: string;
  stderr: string;
  runned_inside?: string;
}

export interface ReturnCodeExitStatus {
  returnCode: number;
}

export interface SignalExitStatus {
  signal: number;
}

export type TimeoutExitStatus = 'timeout';

export type UnknownExitStatus = 'unknown';

export type ExitStatus =
  | ReturnCodeExitStatus
  | SignalExitStatus
  | TimeoutExitStatus
  | UnknownExitStatus
  | number;

export function isExitStatusZero(exitStatus: ExitStatus): boolean {
  return (
    exitStatus == 0 ||
    (typeof exitStatus == 'object' &&
      'returnCode' in exitStatus &&
      exitStatus.returnCode == 0)
  );
}

export function formatExistStatus(exitStatus: ExitStatus): string {
  if (typeof exitStatus == 'string') return exitStatus;
  else if (typeof exitStatus == 'number') return exitStatus.toString();
  else if (typeof exitStatus == 'object') {
    if ('returnCode' in exitStatus) return exitStatus.returnCode.toString();
    else if ('signal' in exitStatus) return `Signal ${exitStatus.signal}`;
    else return JSON.stringify(exitStatus);
  } else return exitStatus;
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
  }
  if (job.stage == 'Finished' && job.resultKind !== 'Accepted') {
    return [{ status: 'OtherError', cnt: 1 }];
  }
  let jobResultCount = Object.keys(job.results).length;
  let res = toPairs(
    mapValues(
      groupBy(job.results, (result) => dashboardTypeToSlider(result.kind)),
      (i) => i.length
    )
  )
    .map(([x, y]) => {
      return { status: x as TestResultKind, cnt: y };
    })
    .sort((x, y) => {
      if (x.status == 'Accepted') {
        if (y.status == 'Accepted') return 0;
        else return -1;
      } else if (y.status == 'Accepted') return 1;
      return x.status.localeCompare(y.status);
    });

  if (jobResultCount < job.tests.length) {
    res.push({ status: 'Waiting', cnt: job.tests.length - jobResultCount });
  }
  return res;
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
