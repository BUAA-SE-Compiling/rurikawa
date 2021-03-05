import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminManageJudgerViewComponent } from './admin-manage-judger-view.component';

describe('AdminManageJudgerViewComponent', () => {
  let component: AdminManageJudgerViewComponent;
  let fixture: ComponentFixture<AdminManageJudgerViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AdminManageJudgerViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminManageJudgerViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
