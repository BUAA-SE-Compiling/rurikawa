import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MainPageComponent } from './main-page/main-page.component';
import { DashBoardComponent } from './dash-board/dash-board.component';
import { TestSuiteViewComponent } from './test-suite-view/test-suite-view.component';
import { JobViewComponent } from './job-view/job-view.component';
import { BaseComponentsModule } from 'src/components/base-components/base-components.module';
import { ItemComponentsModule } from 'src/components/item-components/item-components.module';
import { NotFoundPageComponent } from './not-found-page/not-found-page.component';
import { LoginPageComponent } from './login-page/login-page.component';
import { RegisterPageComponent } from './register-page/register-page.component';
import { RouterModule } from '@angular/router';
import { IconModule } from '@visurel/iconify-angular';
import { MarkdownModule } from 'ngx-markdown';
import { AdminForbiddenPageComponent } from './admin-forbidden-page/admin-forbidden-page.component';
import { AboutPageComponent } from './about-page/about-page.component';

@NgModule({
  declarations: [
    AdminForbiddenPageComponent,
    MainPageComponent,
    DashBoardComponent,
    TestSuiteViewComponent,
    JobViewComponent,
    NotFoundPageComponent,
    LoginPageComponent,
    RegisterPageComponent,
    AboutPageComponent,
  ],
  imports: [
    CommonModule,
    BaseComponentsModule,
    ItemComponentsModule,
    RouterModule,
    IconModule,
    MarkdownModule.forChild(),
  ],
  exports: [
    AdminForbiddenPageComponent,
    MainPageComponent,
    DashBoardComponent,
    TestSuiteViewComponent,
    JobViewComponent,
    NotFoundPageComponent,
    LoginPageComponent,
    RegisterPageComponent,
    AboutPageComponent,
  ],
})
export class DefaultModule {}
