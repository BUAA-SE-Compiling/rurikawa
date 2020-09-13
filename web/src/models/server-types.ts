import { Job } from './job-items';

export interface Profile {
  username: string;
  email: string | undefined;
  studentId: string | undefined;
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
  testGroups: { [key: string]: string[] };
}

export interface DashboardItem {
  suite: TestSuite;
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
  branch?: string;
  testSuite: string;
  tests: string[];
}
