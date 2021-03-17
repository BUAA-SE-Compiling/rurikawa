import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminAnnouncementEditViewComponent } from './admin-announcement-edit-view.component';

describe('AdminAnnouncementEditViewComponent', () => {
  let component: AdminAnnouncementEditViewComponent;
  let fixture: ComponentFixture<AdminAnnouncementEditViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AdminAnnouncementEditViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminAnnouncementEditViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
