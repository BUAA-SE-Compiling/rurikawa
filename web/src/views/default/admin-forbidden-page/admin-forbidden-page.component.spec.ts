import { ComponentFixture, TestBed } from '@angular/core/testing';

import { AdminForbiddenPageComponent } from './admin-forbidden-page.component';

describe('NotFoundPageComponent', () => {
  let component: AdminForbiddenPageComponent;
  let fixture: ComponentFixture<AdminForbiddenPageComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [AdminForbiddenPageComponent],
    }).compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(AdminForbiddenPageComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
