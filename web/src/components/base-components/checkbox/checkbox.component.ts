import { Component, EventEmitter, Input, OnInit, Output } from '@angular/core';

@Component({
  selector: 'checkbox',
  templateUrl: './checkbox.component.html',
  styleUrls: ['./checkbox.component.less'],
})
export class CheckboxComponent implements OnInit {
  constructor() {}

  value_: boolean = false;
  indeterminate_: boolean = false;

  @Input() id: string = '';
  @Input() caption: string = '';
  @Output() valueChange = new EventEmitter<boolean>();
  @Output() indeterminateChange = new EventEmitter<boolean>();

  @Input() set value(value: boolean) {
    this.value_ = value;
    this.valueChange.emit(value);
  }
  get value(): boolean {
    return this.value_;
  }

  @Input() set indeterminate(value: boolean) {
    this.indeterminate_ = value;
    this.indeterminateChange.emit(value);
  }
  get indeterminate(): boolean {
    return this.indeterminate_;
  }

  toggle() {
    this.indeterminate_ = false;
    this.value_ = !this.value_;
    this.valueChange.emit(this.value_);
  }

  ngOnInit(): void {}
}
