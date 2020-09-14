import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminTestSuiteViewComponent } from './admin-test-suite-view.component';

describe('AdminTestSuiteViewComponent', () => {
  let component: AdminTestSuiteViewComponent;
  let fixture: ComponentFixture<AdminTestSuiteViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AdminTestSuiteViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminTestSuiteViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
