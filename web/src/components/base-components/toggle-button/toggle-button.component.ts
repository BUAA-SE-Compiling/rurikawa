import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';

@Component({
  selector: 'app-toggle-button',
  templateUrl: './toggle-button.component.html',
  styleUrls: ['./toggle-button.component.less'],
})
export class ToggleButtonComponent implements OnInit {
  constructor() {}

  @Input()
  active: boolean = false;
  @Output() activeChanged = new EventEmitter<boolean>();

  toggle() {
    this.active = !this.active;
    this.activeChanged.emit(this.active);
  }

  ngOnInit(): void {}
}
