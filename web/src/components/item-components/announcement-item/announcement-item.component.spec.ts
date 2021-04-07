import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AnnouncementItemComponent } from './announcement-item.component';

describe('AnnouncementItemComponent', () => {
  let component: AnnouncementItemComponent;
  let fixture: ComponentFixture<AnnouncementItemComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AnnouncementItemComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AnnouncementItemComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
