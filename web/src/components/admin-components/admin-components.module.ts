import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DashboardItemComponent } from './dashboard-item/dashboard-item.component';



@NgModule({
  declarations: [DashboardItemComponent],
  imports: [
    CommonModule
  ],
  exports: [DashboardItemComponent]
})
export class AdminComponentsModule { }
