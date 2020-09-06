import {
  Component,
  OnInit,
  Input,
  ElementRef,
  ViewChild,
  OnChanges,
  SimpleChanges,
} from '@angular/core';

export type SliderItemKind =
  | 'accept'
  | 'error'
  | 'warn'
  | 'info'
  | 'info-alt'
  | 'disable'
  | 'cancel';

export interface SliderItem {
  kind: SliderItemKind;
  num: number;
}

interface InternalSliderItem {
  style: { [kl: string]: any };
}

function kindToVar(kind: SliderItemKind): string {
  switch (kind) {
    case 'accept':
      return '--success-color';
    case 'error':
      return '--error-color';
    case 'warn':
      return '--warning-color';
    case 'info':
      return '--info-color';
    case 'info-alt':
      return '--info-alt-color';
    case 'disable':
      return '--disabled-color';
    case 'cancel':
      return '--gray-color';
  }
}

@Component({
  selector: 'app-slider-view',
  templateUrl: './slider-view.component.html',
  styleUrls: ['./slider-view.component.styl'],
})
export class SliderViewComponent implements OnInit {
  constructor() {}

  @Input()
  items: SliderItem[];

  @Input()
  gapWidth: number = 4;

  @Input()
  height: number = 4;

  @ViewChild('canvas')
  canvas: ElementRef;

  get sliderItems(): InternalSliderItem[] {
    return this.items.map((i, idx) => {
      let style = {
        'background-color': 'var(' + kindToVar(i.kind) + ')',
        'flex-grow': i.num,
        'margin-left': idx === 0 ? '0px' : this.gapWidth + 'px',
        'margin-right':
          idx === this.items.length ? '0px' : this.gapWidth + 'px',
      };
      return {
        style,
      };
    });
  }

  get wrapperStyle() {
    return {
      height: this.height + 'px',
    };
  }

  ngOnInit(): void {}
}
