export type TestStatus = 'ac' | 'wa' | 'tle' | 're' | 'ce' | 'oe';

interface DashboardItem {
  name: string;
  endTime: Date;
  finishedItem: number;
  completedItem: number;
  status: { status: TestStatus; cnt: number }[];
}
