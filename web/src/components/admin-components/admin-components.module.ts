import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AdminDashboardItemComponent } from './dashboard-item/dashboard-item.component';
import { UserItemComponent } from './user-item/user-item.component';
import { BaseComponentsModule } from '../base-components/base-components.module';

@NgModule({
  declarations: [AdminDashboardItemComponent, UserItemComponent],
  imports: [CommonModule, BaseComponentsModule],
  exports: [AdminDashboardItemComponent, UserItemComponent],
})
export class AdminComponentsModule {}
