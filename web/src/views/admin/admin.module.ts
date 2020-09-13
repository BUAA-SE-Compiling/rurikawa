import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';

import { RouterModule, Routes } from '@angular/router';

import { BaseComponentsModule } from 'src/components/base-components/base-components.module';
import { ItemComponentsModule } from 'src/components/item-components/item-components.module';
import { DashboardComponent } from './dashboard/dashboard.component';
import { InitDatabaseComponent } from './init-database/init-database.component';

const routes: Routes = [
  {
    path: '',
    component: DashboardComponent,
  },
  {
    path: 'init-db',
    component: InitDatabaseComponent,
  },
];

@NgModule({
  declarations: [DashboardComponent, InitDatabaseComponent],
  imports: [
    CommonModule,
    BaseComponentsModule,
    ItemComponentsModule,
    RouterModule.forChild(routes),
  ],
  exports: [DashboardComponent, InitDatabaseComponent],
})
export class AdminModule {}
