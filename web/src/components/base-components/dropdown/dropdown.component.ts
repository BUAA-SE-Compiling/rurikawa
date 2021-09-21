import {
  Component,
  OnInit,
  Input,
  Output,
  EventEmitter,
  ElementRef,
  ViewChild,
} from '@angular/core';

type style = undefined | 'success' | 'warn' | 'error';
type borderType = 'all' | 'underline' | 'none';

@Component({
  // tslint:disable-next-line: component-selector
  selector: 'dropdown',
  templateUrl: './dropdown.component.html',
  styleUrls: ['./dropdown.component.less'],
})
export class DropdownComponent implements OnInit {
  @Input() type: string = 'text';
  @Input() style: style;
  @Input() border: borderType = 'all';
  @Input() caption: string = '';
  @Input() message: string = '';
  @Input() disabled: boolean = false;

  _value: string[] = [];

  @Input() set value(v: string | string[]) {
    if (this.multiple) {
      let val: string[];

      if (typeof v == 'string') val = [v];
      else val = v;

      this._value = val;
      // this.valueChange.emit(val);
    } else {
      let val: string;
      if (v instanceof Array) val = v[0];
      else val = v;
      this._value = [val];
      // this.valueChange.emit(this._value);
    }
  }
  get value() {
    return this._value;
  }

  @Input() multiple: boolean = false;
  @Input() icon: any | undefined;
  @Input() iconSize: number = 16;
  @Output() valueChange = new EventEmitter<string | string[]>();

  @ViewChild('input') input: ElementRef;

  get styleClass() {
    let styleClass: any = {};
    if (this.style) {
      styleClass[this.style] = true;
    }
    if (this.disabled) {
      styleClass.disabled = true;
    }
    return styleClass;
  }

  get inputClass() {
    let inputClass: any = {};
    inputClass['border-' + this.border] = true;

    return inputClass;
  }

  ngModelChange() {
    if (this.multiple) this.valueChange.emit(this._value);
    else {
      if (this._value.length > 1) {
      }
      this.valueChange.emit(this._value[0]);
    }
  }

  constructor() {}

  focus() {
    this.input.nativeElement.focus();
  }

  ngOnInit(): void {}
}
