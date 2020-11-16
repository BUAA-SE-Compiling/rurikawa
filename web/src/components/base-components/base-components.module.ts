import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { NavbarComponent } from './navbar/navbar.component';
import { SliderViewComponent } from './slider-view/slider-view.component';
import { RouterModule } from '@angular/router';
import { TextboxComponent } from './textbox/textbox.component';
import { IconModule } from '@visurel/iconify-angular';
import { FileUploadAreaComponent } from './file-upload-area/file-upload-area.component';
import { FormsModule } from '@angular/forms';
import { CollapseBoxComponent } from './collapse-box/collapse-box.component';
import { BackBtnComponent } from './back-btn/back-btn.component';
import { ToggleButtonComponent } from './toggle-button/toggle-button.component';

@NgModule({
  declarations: [
    NavbarComponent,
    SliderViewComponent,
    TextboxComponent,
    FileUploadAreaComponent,
    CollapseBoxComponent,
    BackBtnComponent,
    ToggleButtonComponent,
  ],
  imports: [CommonModule, RouterModule, IconModule, FormsModule],
  exports: [
    NavbarComponent,
    SliderViewComponent,
    TextboxComponent,
    FileUploadAreaComponent,
    CollapseBoxComponent,
    BackBtnComponent,
    ToggleButtonComponent,
  ],
})
export class BaseComponentsModule {}
