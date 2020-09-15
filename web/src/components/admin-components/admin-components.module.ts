import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { AdminDashboardItemComponent } from './dashboard-item/dashboard-item.component';

@NgModule({
  declarations: [AdminDashboardItemComponent],
  imports: [CommonModule],
  exports: [AdminDashboardItemComponent],
})
export class AdminComponentsModule {}
