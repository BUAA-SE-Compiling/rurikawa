import { Job } from './job-items';

export enum UserKind {
  User = 'User',
  Admin = 'Admin',
  Root = 'Root',
}

export interface Profile {
  username: string;
  email: string | undefined;
  studentId: string | undefined;
}

export interface AccountAndProfile {
  username: string;
  kind: UserKind;
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
  scoringMode: ScoringMode;
  testGroups: { [key: string]: TestCaseDefinition[] };
}

export enum ScoringMode {
  /** The basic scoring mode, display `{passedCases}/{totalCases}` */
  Basic = 'Basic',
  /** Floating scoring mode. Displays `{currentScore}/{totalScore}` */
  Floating = 'Floating',
}

export interface TestCaseDefinition {
  name: string;
  hasOut: boolean;
  shouldFail: boolean;
  baseScore?: number;
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

export interface JudgerStatus {
  count: number;
  connected: number;
  running: number;
}
