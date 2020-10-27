import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { DashBoardComponent } from 'src/views/default/dash-board/dash-board.component';
import { DefaultModule } from 'src/views/default/default.module';
import { MainPageComponent } from 'src/views/default/main-page/main-page.component';
import { TestSuiteViewComponent } from 'src/views/default/test-suite-view/test-suite-view.component';
import { JobViewComponent } from 'src/views/default/job-view/job-view.component';
import { NotFoundPageComponent } from 'src/views/default/not-found-page/not-found-page.component';
import { RegisterPageComponent } from 'src/views/default/register-page/register-page.component';
import { LoginPageComponent } from 'src/views/default/login-page/login-page.component';
import {
  LoginGuard,
  AdminLoginGuard,
  NotLoggedInGuard,
  NotLoggedInRedirectToDashboardGuard,
} from 'src/services/account_service';
import { AdminForbiddenPageComponent } from 'src/views/default/admin-forbidden-page/admin-forbidden-page.component';
import { AboutPageComponent } from 'src/views/default/about-page/about-page.component';
import { SettingsViewComponent } from 'src/views/default/settings-view/settings-view.component';
import { JobTestcaseViewComponent } from 'src/views/default/job-testcase-view/job-testcase-view.component';

const routes: Routes = [
  {
    path: '',
    pathMatch: 'full',
    component: MainPageComponent,
    canActivate: [NotLoggedInRedirectToDashboardGuard],
  },
  {
    path: 'dashboard',
    component: DashBoardComponent,
    canActivate: [LoginGuard],
  },
  {
    path: 'suite/:id',
    component: TestSuiteViewComponent,
    canActivate: [LoginGuard],
  },
  {
    path: 'job/:id',
    component: JobViewComponent,
    canActivate: [LoginGuard],
  },
  {
    path: 'job/:id/case/:case',
    component: JobTestcaseViewComponent,
    canActivate: [LoginGuard],
  },
  {
    path: 'register',
    component: RegisterPageComponent,
  },
  {
    path: 'login',
    component: LoginPageComponent,
  },
  {
    path: 'about',
    component: AboutPageComponent,
  },
  {
    path: 'settings',
    component: SettingsViewComponent,
  },
  {
    path: 'admin',
    loadChildren: () =>
      import('../views/admin/admin.module').then((m) => m.AdminModule),
    canActivate: [AdminLoginGuard],
    canActivateChild: [AdminLoginGuard],
  },
  {
    path: '403',
    pathMatch: 'full',
    component: AdminForbiddenPageComponent,
  },
  {
    path: '**',
    component: NotFoundPageComponent,
  },
];

@NgModule({
  imports: [RouterModule.forRoot(routes), DefaultModule],
  exports: [RouterModule],
})
export class AppRoutingModule {}
