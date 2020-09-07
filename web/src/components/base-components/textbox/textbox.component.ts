import { Component, OnInit, Input, Output, EventEmitter } from '@angular/core';

type style = undefined | 'success' | 'warn' | 'error';

@Component({
  // tslint:disable-next-line: component-selector
  selector: 'textbox',
  templateUrl: './textbox.component.html',
  styleUrls: ['./textbox.component.styl'],
})
export class TextboxComponent implements OnInit {
  @Input() type: string = 'text';
  @Input() style: style;
  @Input() caption: string = '';
  @Input() placeholder: string = '';
  @Input() message: string = '';

  @Input() value: string;
  @Output() valueChange = new EventEmitter<string>();

  constructor() {}

  ngOnInit(): void {}
}
