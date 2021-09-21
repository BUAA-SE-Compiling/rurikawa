import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminAddUserViewComponent } from './admin-add-user-view.component';

describe('AdminAddUserViewComponent', () => {
  let component: AdminAddUserViewComponent;
  let fixture: ComponentFixture<AdminAddUserViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AdminAddUserViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminAddUserViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
