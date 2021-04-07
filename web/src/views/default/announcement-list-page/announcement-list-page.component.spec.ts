import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AnnouncementListPageComponent } from './announcement-list-page.component';

describe('AnnouncementListPageComponent', () => {
  let component: AnnouncementListPageComponent;
  let fixture: ComponentFixture<AnnouncementListPageComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AnnouncementListPageComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AnnouncementListPageComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
