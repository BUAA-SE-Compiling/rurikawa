import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MainPageComponent } from './main-page/main-page.component';
import { DashBoardComponent } from './dash-board/dash-board.component';
import { TestSuiteViewComponent } from './test-suite-view/test-suite-view.component';
import { JobViewComponent } from './job-view/job-view.component';
import { BaseComponentsModule } from 'src/components/base-components/base-components.module';

@NgModule({
  declarations: [
    MainPageComponent,
    DashBoardComponent,
    TestSuiteViewComponent,
    JobViewComponent,
  ],
  imports: [CommonModule, BaseComponentsModule],
  exports: [
    MainPageComponent,
    DashBoardComponent,
    TestSuiteViewComponent,
    JobViewComponent,
  ],
})
export class DefaultModule {}
