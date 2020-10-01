import { ComponentFixture, TestBed } from '@angular/core/testing';

import { FileUploadAreaComponent } from './file-upload-area.component';

describe('FileUploadAreaComponent', () => {
  let component: FileUploadAreaComponent;
  let fixture: ComponentFixture<FileUploadAreaComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ FileUploadAreaComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(FileUploadAreaComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
