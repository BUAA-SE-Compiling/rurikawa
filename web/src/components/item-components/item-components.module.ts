import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DashboardItemComponentComponent } from './dashboard-item-component/dashboard-item-component.component';
import { BaseComponentsModule } from '../base-components/base-components.module';
import { JobItemComponent } from './job-item/job-item.component';
import { RouterModule } from '@angular/router';
import { JobTestItemComponent } from './job-test-item/job-test-item.component';
import { AnnouncementItemComponent } from './announcement-item/announcement-item.component';
import { MarkdownModule } from 'ngx-markdown';
import { IconModule } from '@visurel/iconify-angular';

@NgModule({
  declarations: [
    DashboardItemComponentComponent,
    JobItemComponent,
    JobTestItemComponent,
    AnnouncementItemComponent,
  ],
  imports: [
    CommonModule,
    BaseComponentsModule,
    RouterModule,
    MarkdownModule,
    IconModule,
  ],
  exports: [
    DashboardItemComponentComponent,
    JobItemComponent,
    JobTestItemComponent,
    AnnouncementItemComponent,
  ],
})
export class ItemComponentsModule {}
