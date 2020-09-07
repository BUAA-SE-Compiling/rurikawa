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

  @Input() value: string = '';
  @Input() icon: any | undefined;
  @Input() iconSize: number = 16;
  @Output() valueChange = new EventEmitter<string>();

  @ViewChild('input') input: ElementRef;

  constructor() {}

  focus() {
    this.input.nativeElement.focus();
  }

  ngOnInit(): void {}
}
