import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminAnnouncementViewComponent } from './admin-announcement-view.component';

describe('AdminAnnouncementViewComponent', () => {
  let component: AdminAnnouncementViewComponent;
  let fixture: ComponentFixture<AdminAnnouncementViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AdminAnnouncementViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminAnnouncementViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
