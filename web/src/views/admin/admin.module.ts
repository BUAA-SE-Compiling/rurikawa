import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';

import { RouterModule, Routes } from '@angular/router';

import { BaseComponentsModule } from 'src/components/base-components/base-components.module';
import { ItemComponentsModule } from 'src/components/item-components/item-components.module';
import { DashboardComponent } from './dashboard/dashboard.component';
import { InitDatabaseComponent } from './init-database/init-database.component';
import { AdminTestSuiteViewComponent } from './admin-test-suite-view/admin-test-suite-view.component';

const routes: Routes = [
  {
    path: '',
    component: DashboardComponent,
  },
  {
    path: 'suite/:id',
    component: AdminTestSuiteViewComponent,
  },
  {
    path: 'init-db',
    component: InitDatabaseComponent,
  },
];

@NgModule({
  declarations: [
    DashboardComponent,
    InitDatabaseComponent,
    AdminTestSuiteViewComponent,
  ],
  imports: [
    CommonModule,
    BaseComponentsModule,
    ItemComponentsModule,
    RouterModule.forChild(routes),
  ],
  exports: [
    DashboardComponent,
    InitDatabaseComponent,
    AdminTestSuiteViewComponent,
  ],
})
export class AdminModule {}
