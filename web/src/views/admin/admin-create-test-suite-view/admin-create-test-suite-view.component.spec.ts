import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminCreateTestSuiteViewComponent } from './admin-create-test-suite-view.component';

describe('AdminCreateTestSuiteViewComponent', () => {
  let component: AdminCreateTestSuiteViewComponent;
  let fixture: ComponentFixture<AdminCreateTestSuiteViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ AdminCreateTestSuiteViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminCreateTestSuiteViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
