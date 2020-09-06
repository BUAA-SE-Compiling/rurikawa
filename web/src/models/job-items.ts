import { SliderItemKind } from 'src/components/base-components/slider-view/slider-view.component';

export type TestStatus = 'ac' | 'wa' | 'tle' | 're' | 'ce' | 'oe' | 'nt';

export function dashboardTypeToSlider(item: TestStatus): SliderItemKind {
  switch (item) {
    case 'ac':
      return 'accept';
    case 'wa':
      return 'error';
    case 'ce':
      return 'disable';
    case 're':
      return 'error';
    case 'oe':
      return 'disable';
    case 'tle':
      return 'warn';
    case 'nt':
      return 'cancel';
  }
}

export interface DashboardItem {
  id: string;
  name: string;
  endTime: Date;
  finishedItem: number;
  totalItem: number;
  status: { status: TestStatus; cnt: number }[];
}

export interface JobItem {
  id: string;
  time: Date;
  finishedItem: number;
  totalItem: number;
  status: { status: TestStatus; cnt: number }[];
}
