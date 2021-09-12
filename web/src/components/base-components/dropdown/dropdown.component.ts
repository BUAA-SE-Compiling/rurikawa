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

  _value: string = '';

  @Input() set value(v) {
    this._value = v;
    this.valueChange.emit(this._value);
  }
  get value() {
    return this._value;
  }

  @Input() icon: any | undefined;
  @Input() iconSize: number = 16;
  @Output() valueChange = new EventEmitter<string>();

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

  constructor() {}

  focus() {
    this.input.nativeElement.focus();
  }

  ngOnInit(): void {}
}
