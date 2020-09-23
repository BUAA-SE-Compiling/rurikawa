import { Job, JobStage, JobResultKind, TestResult } from './job-items';
import { Dictionary } from 'lodash';

export type ServerMessageKind =
  | 'new_job_s'
  | 'job_status_s'
  | 'judger_status_s'
  | 'test_output_s';

export type ClientMessageKind = 'sub_c';

export interface WsApiMsg {
  _t: ServerMessageKind | ClientMessageKind;
}

export interface NewJobUpdateMsg extends WsApiMsg {
  _t: 'new_job_s';
  job: Job;
}

export interface JobStatusUpdateMsg extends WsApiMsg {
  _t: 'job_status_s';
  jobId: string;
  buildStream?: string;
  stage?: JobStage;
  jobResult?: JobResultKind;
  testResult?: Dictionary<TestResult>;
}

export interface SubscribeMsg extends WsApiMsg {
  _t: 'sub_c';
  sub: boolean;
  jobs?: string[];
  suites?: string[];
}
