import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';

@Component({
  selector: 'app-collapse-box',
  templateUrl: './collapse-box.component.html',
  styleUrls: ['./collapse-box.component.less'],
})
export class CollapseBoxComponent implements OnInit {
  constructor() {}

  @Input() set initialState(value: boolean) {
    if (!this.initialized) {
      this.collapsed = value;
      this.initialized = true;
      if (value === false) {
        this.loadContents.emit();
        this.contentLoaded = true;
      }
    }
  }

  @Input() title: string;
  initialized = false;
  contentLoaded = false;

  collapsed: boolean = true;

  @Output() loadContents = new EventEmitter();

  toggle() {
    this.collapsed = !this.collapsed;
    if (!this.contentLoaded && !this.collapsed) {
      this.loadContents.emit();
    }
  }

  ngOnInit(): void {}
}
