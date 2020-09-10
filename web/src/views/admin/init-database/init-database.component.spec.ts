import { ComponentFixture, TestBed } from '@angular/core/testing';

import { InitDatabaseComponent } from './init-database.component';

describe('InitDatabaseComponent', () => {
  let component: InitDatabaseComponent;
  let fixture: ComponentFixture<InitDatabaseComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ InitDatabaseComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(InitDatabaseComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
