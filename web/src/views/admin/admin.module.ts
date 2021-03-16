import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';

import { RouterModule, Routes } from '@angular/router';

import { BaseComponentsModule } from 'src/components/base-components/base-components.module';
import { ItemComponentsModule } from 'src/components/item-components/item-components.module';
import { DashboardComponent } from './dashboard/dashboard.component';
import { InitDatabaseComponent } from './init-database/init-database.component';
import { AdminTestSuiteViewComponent } from './admin-test-suite-view/admin-test-suite-view.component';
import { AdminComponentsModule } from 'src/components/admin-components/admin-components.module';
import { AdminCreateTestSuiteViewComponent } from './admin-create-test-suite-view/admin-create-test-suite-view.component';
import { HttpClientModule } from '@angular/common/http';
import { AdminManageJudgerViewComponent } from './admin-manage-judger-view/admin-manage-judger-view.component';
import { AdminAnnouncementViewComponent } from './admin-announcement-view/admin-announcement-view.component';

const routes: Routes = [
  {
    path: '',
    component: DashboardComponent,
  },
  {
    path: 'suite/create',
    component: AdminCreateTestSuiteViewComponent,
  },
  {
    path: 'suite/:id',
    component: AdminTestSuiteViewComponent,
  },
  {
    path: 'init-db',
    component: InitDatabaseComponent,
  },
  {
    path: 'judger',
    component: AdminManageJudgerViewComponent,
  },
];

@NgModule({
  declarations: [
    DashboardComponent,
    InitDatabaseComponent,
    AdminTestSuiteViewComponent,
    AdminCreateTestSuiteViewComponent,
    AdminManageJudgerViewComponent,
    AdminAnnouncementViewComponent,
  ],
  imports: [
    CommonModule,
    BaseComponentsModule,
    ItemComponentsModule,
    AdminComponentsModule,
    RouterModule.forChild(routes),
  ],
  exports: [
    DashboardComponent,
    InitDatabaseComponent,
    AdminTestSuiteViewComponent,
    AdminCreateTestSuiteViewComponent,
    AdminManageJudgerViewComponent,
    AdminAnnouncementViewComponent,
  ],
})
export class AdminModule {}
