import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DashboardItemComponentComponent } from './dashboard-item-component/dashboard-item-component.component';

@NgModule({
  declarations: [DashboardItemComponentComponent],
  imports: [CommonModule],
  exports: [DashboardItemComponentComponent],
})
export class ItemComponentsModule {}
