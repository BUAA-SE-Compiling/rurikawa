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
  selector: 'textbox',
  templateUrl: './textbox.component.html',
  styleUrls: ['./textbox.component.less'],
})
export class TextboxComponent implements OnInit {
  @Input() multiline: boolean = false;

  @Input() type: string = 'text';
  @Input() style: style;
  @Input() border: borderType = 'all';
  @Input() caption: string = '';
  @Input() placeholder: string = '';
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

  @Input() id: string | undefined;
  @Input() icon: any | undefined;
  @Input() iconSize: number = 16;
  @Output() valueChange = new EventEmitter<string>();
  @Output() enterKeyPress = new EventEmitter<KeyboardEvent>();
  @Output() keyPress = new EventEmitter<KeyboardEvent>();
  @Output() keyDown = new EventEmitter<KeyboardEvent>();
  @Output() keyUp = new EventEmitter<KeyboardEvent>();

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
