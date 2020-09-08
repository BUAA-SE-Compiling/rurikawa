import { ComponentFixture, TestBed } from '@angular/core/testing';

import { JobTestItemComponent } from './job-test-item.component';

describe('JobTestItemComponent', () => {
  let component: JobTestItemComponent;
  let fixture: ComponentFixture<JobTestItemComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ JobTestItemComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(JobTestItemComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
