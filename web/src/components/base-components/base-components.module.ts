import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NavbarComponent } from './navbar/navbar.component';
import { SliderViewComponent } from './slider-view/slider-view.component';

@NgModule({
  declarations: [NavbarComponent, SliderViewComponent],
  imports: [CommonModule],
  exports: [NavbarComponent, SliderViewComponent],
})
export class BaseComponentsModule {}
