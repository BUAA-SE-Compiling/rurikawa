import { Job } from './job-items';

export interface Profile {
  username: string;
  email: string | undefined;
  studentId: string | undefined;
}

export interface PartialTestSuite {
  id: string;
  title: string;
}

export interface TestSuite {
  id: string;
  name: string;
  title: string;
  description: string;
  tags?: string[];
  packageFileId: string;
  timeLimit?: number;
  memoryLimit?: number;
  isPublic: boolean;
  startTime: Date;
  endTime: Date;
  testGroups: { [key: string]: string[] };
}

export interface DashboardItem {
  suite: PartialTestSuite;
  job: Job;
}

export type AnnouncementKind =
  | 'Generic'
  | 'Info'
  | 'Warn'
  | 'Error'
  | 'Success';

export interface Announcement {
  id: string;
  title: string;
  body: string;
  sender: string;
  sendTime: Date;
  tags: string[];
  kind: AnnouncementKind;
}

export interface JudgerEntry {
  id: string;
  alternateName: string | undefined;
  tags: string[] | undefined;
  acceptUntaggedJobs: boolean;
}

export interface NewJobMessage {
  repo: string;
  ref?: string;
  testSuite: string;
  tests: string[];
}

export interface JobBuildOutput {
  output?: string;
  error?: string;
}
