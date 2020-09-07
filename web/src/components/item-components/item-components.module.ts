import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DashboardItemComponentComponent } from './dashboard-item-component/dashboard-item-component.component';
import { BaseComponentsModule } from '../base-components/base-components.module';
import { JobItemComponent } from './job-item/job-item.component';
import { RouterModule } from '@angular/router';

@NgModule({
  declarations: [DashboardItemComponentComponent, JobItemComponent],
  imports: [CommonModule, BaseComponentsModule, RouterModule],
  exports: [DashboardItemComponentComponent, JobItemComponent],
})
export class ItemComponentsModule {}
