import { ComponentFixture, TestBed } from '@angular/core/testing';

import { DashboardItemComponentComponent } from './dashboard-item-component.component';

describe('DashboardItemComponentComponent', () => {
  let component: DashboardItemComponentComponent;
  let fixture: ComponentFixture<DashboardItemComponentComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ DashboardItemComponentComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(DashboardItemComponentComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
