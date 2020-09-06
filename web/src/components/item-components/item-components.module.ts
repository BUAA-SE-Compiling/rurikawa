import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DashboardItemComponentComponent } from './dashboard-item-component/dashboard-item-component.component';
import { BaseComponentsModule } from '../base-components/base-components.module';

@NgModule({
  declarations: [DashboardItemComponentComponent],
  imports: [CommonModule, BaseComponentsModule],
  exports: [DashboardItemComponentComponent],
})
export class ItemComponentsModule {}
