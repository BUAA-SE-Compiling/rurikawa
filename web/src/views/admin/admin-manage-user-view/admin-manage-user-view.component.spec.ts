import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminManageUserViewComponent } from './admin-manage-user-view.component';

describe('AdminManageUserViewComponent', () => {
  let component: AdminManageUserViewComponent;
  let fixture: ComponentFixture<AdminManageUserViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AdminManageUserViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminManageUserViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
