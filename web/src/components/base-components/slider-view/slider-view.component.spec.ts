import { ComponentFixture, TestBed } from '@angular/core/testing';

import { SliderViewComponent } from './slider-view.component';

describe('SliderViewComponent', () => {
  let component: SliderViewComponent;
  let fixture: ComponentFixture<SliderViewComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ SliderViewComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(SliderViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
