import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';

type style = undefined | 'warn' | 'error';

@Component({
  // tslint:disable-next-line: component-selector
  selector: 'textbox',
  templateUrl: './textbox.component.html',
  styleUrls: ['./textbox.component.styl'],
})
export class TextboxComponent implements OnInit {
  @Input() type: string = 'text';
  @Input() style: style;
  @Input() placeholder: string | undefined;

  // tslint:disable-next-line: no-input-rename
  @Input('value') underlyingValue: string;
  @Output() valueChange = new EventEmitter<string>();

  get value() {
    return this.underlyingValue;
  }
  set value(val) {
    this.underlyingValue = val;
    this.valueChange.emit(val);
  }

  constructor() {}

  ngOnInit(): void {}
}
