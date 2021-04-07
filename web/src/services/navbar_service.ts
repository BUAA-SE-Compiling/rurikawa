import { Injectable } from '@angular/core';
import {
  ActivatedRoute,
  NavigationEnd,
  NavigationStart,
  Router,
  RouterEvent,
} from '@angular/router';

@Injectable({ providedIn: 'root' })
export class NavbarService {
  constructor(private router: Router) {
    router.events.subscribe((ev) => {
      if (ev instanceof NavigationStart) {
        if (ev.navigationTrigger == 'popstate') {
          this.preparingToPop = ev.id;
        }
      }
      if (ev instanceof NavigationEnd) {
        if (this.preparingToPop == ev.id && this.popWhenNavigateBack) {
          this.popStyle();
        }
      }
    });
  }

  get currentStyle() {
    if (this.styles.length == 0) return NAVBAR_DEFAULT_STYLE;
    else return this.styles[this.styles.length - 1];
  }

  get popWhenNavigateBack() {
    if (this.navBackBehavior.length == 0) return NAVBAR_DEFAULT_STYLE;
    else return this.navBackBehavior[this.navBackBehavior.length - 1];
  }

  preparingToPop = undefined;

  styles: NavbarStyle[] = [];
  navBackBehavior: boolean[] = [];

  public pushStyle(style: NavbarStyle, popOnNavigateBack: boolean = true) {
    this.styles.push(style);
    this.navBackBehavior.push(popOnNavigateBack);
  }

  public popStyle() {
    this.styles.pop();
    this.navBackBehavior.pop();
  }
}

export interface NavbarStyle {
  is_admin_mode: boolean;
  color: NavbarColorScheme;
  display: NavbarDisplayKind;
}

export enum NavbarDisplayKind {
  Normal,
  Collapse,
  None,
}

export enum NavbarColorScheme {
  Default,
  Accent,
}

export const NAVBAR_DEFAULT_STYLE = {
  is_admin_mode: false,
  color: NavbarColorScheme.Default,
  display: NavbarDisplayKind.Normal,
};
