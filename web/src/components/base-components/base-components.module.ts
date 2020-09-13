import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NavbarComponent } from './navbar/navbar.component';
import { SliderViewComponent } from './slider-view/slider-view.component';
import { RouterModule } from '@angular/router';
import { TextboxComponent } from './textbox/textbox.component';
import { IconModule } from '@visurel/iconify-angular'
@NgModule({
  declarations: [NavbarComponent, SliderViewComponent, TextboxComponent],
  imports: [CommonModule, RouterModule,IconModule],
  exports: [NavbarComponent, SliderViewComponent, TextboxComponent],
})
export class BaseComponentsModule {}
