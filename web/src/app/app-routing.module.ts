import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { DashBoardComponent } from 'src/views/default/dash-board/dash-board.component';
import { DefaultModule } from 'src/views/default/default.module';

const routes: Routes = [
  {
    path: '',
    component: DashBoardComponent,
  },
];

@NgModule({
  imports: [RouterModule.forRoot(routes), DefaultModule],
  exports: [RouterModule],
})
export class AppRoutingModule {}
