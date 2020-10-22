import { ComponentFixture, TestBed } from '@angular/core/testing';

import { JobTestcaseViewComponent } from './job-testcase-view.component';

describe('JobTestcaseViewComponent', () => {
  let component: JobTestcaseViewComponent;
  let fixture: ComponentFixture<JobTestcaseViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ JobTestcaseViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(JobTestcaseViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
