export type TestStatus = 'ac' | 'wa' | 'tle' | 're' | 'ce' | 'oe';

export interface DashboardItem {
  id: string;
  name: string;
  endTime: Date;
  finishedItem: number;
  completedItem: number;
  status: { status: TestStatus; cnt: number }[];
}
