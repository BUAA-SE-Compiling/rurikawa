import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NavbarComponent } from './navbar/navbar.component';
import { SliderViewComponent } from './slider-view/slider-view.component';
import { RouterModule } from '@angular/router';
import { TextboxComponent } from './textbox/textbox.component';
import { IconModule } from '@visurel/iconify-angular';
import { FileUploadAreaComponent } from './file-upload-area/file-upload-area.component';
import { FormsModule } from '@angular/forms';

@NgModule({
  declarations: [
    NavbarComponent,
    SliderViewComponent,
    TextboxComponent,
    FileUploadAreaComponent,
  ],
  imports: [CommonModule, RouterModule, IconModule, FormsModule],
  exports: [
    NavbarComponent,
    SliderViewComponent,
    TextboxComponent,
    FileUploadAreaComponent,
  ],
})
export class BaseComponentsModule {}
