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
import { AdminAnnouncementEditViewComponent } from './admin-announcement-edit-view/admin-announcement-edit-view.component';
import { NuMarkdownModule } from '@ng-util/markdown';
import { FormsModule } from '@angular/forms';
import { AdminManageUserViewComponent } from './admin-manage-user-view/admin-manage-user-view.component';
import { AdminAddUserViewComponent } from './admin-add-user-view/admin-add-user-view.component';

const routes: Routes = [
  {
    path: '',
    component: DashboardComponent,
  },
  {
    path: 'announcement/new',
    data: { new: true },
    component: AdminAnnouncementEditViewComponent,
  },
  {
    path: 'announcement/edit/:id',
    data: { new: false },
    component: AdminAnnouncementEditViewComponent,
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
  {
    path: 'user/add',
    component: AdminAddUserViewComponent,
  },
  {
    path: 'user',
    component: AdminManageUserViewComponent,
  },
];

@NgModule({
  declarations: [
    DashboardComponent,
    InitDatabaseComponent,
    AdminTestSuiteViewComponent,
    AdminCreateTestSuiteViewComponent,
    AdminManageJudgerViewComponent,
    AdminAnnouncementEditViewComponent,
    AdminManageUserViewComponent,
    AdminAddUserViewComponent,
  ],
  imports: [
    CommonModule,
    BaseComponentsModule,
    ItemComponentsModule,
    AdminComponentsModule,
    FormsModule,
    RouterModule.forChild(routes),
    NuMarkdownModule,
  ],
  exports: [
    DashboardComponent,
    InitDatabaseComponent,
    AdminTestSuiteViewComponent,
    AdminCreateTestSuiteViewComponent,
    AdminManageJudgerViewComponent,
    AdminAnnouncementEditViewComponent,
    AdminManageUserViewComponent,
    AdminAddUserViewComponent,
  ],
})
export class AdminModule {}
