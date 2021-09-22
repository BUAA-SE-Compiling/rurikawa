import { Injectable } from '@angular/core';
import {
  ActivatedRoute,
  NavigationCancel,
  NavigationEnd,
  NavigationError,
  NavigationStart,
  Router,
  RouterEvent,
} from '@angular/router';
import { upperFirst } from 'lodash';

@Injectable({ providedIn: 'root' })
export class NavbarService {
  constructor(private router: Router) {
    router.events.subscribe((ev) => {
      if (ev instanceof NavigationStart) {
        if (ev.navigationTrigger == 'popstate') {
          this.preparingToPop = ev.id;
          this.navigating = true;
        } else {
        }
      }
      if (ev instanceof NavigationEnd) {
        if (this.preparingToPop == ev.id && this.popWhenNavigateBack) {
          this.popStyle();
          this.navigating = false;
          this.performDeferredPush();
        }
      }
      if (ev instanceof NavigationCancel || ev instanceof NavigationError) {
        this.navigating = false;
        this.performDeferredPush();
      }
    });
  }

  get currentStyle() {
    if (this.styles.length == 0) return NAVBAR_DEFAULT_STYLE;
    else return this.styles[this.styles.length - 1];
  }

  get popWhenNavigateBack() {
    if (this.navBackBehavior.length == 0) return false;
    else return this.navBackBehavior[this.navBackBehavior.length - 1];
  }

  preparingToPop = undefined;
  navigating = false;
  deferredPush?: [NavbarStyle, boolean] = undefined;

  styles: NavbarStyle[] = [];
  navBackBehavior: boolean[] = [];

  private performDeferredPush() {
    if (!this.deferredPush) return;
    let [style, pop] = this.deferredPush;
    this.pushStyle(style, pop);
    this.deferredPush = undefined;
  }

  public pushStyle(style: NavbarStyle, popOnNavigateBack: boolean = true) {
    if (this.navigating) {
      this.deferredPush = [style, popOnNavigateBack];
      return;
    }
    this.styles.push(style);
    this.navBackBehavior.push(popOnNavigateBack);
    console.log('pushed new style', style);
  }

  public popStyle() {
    this.styles.pop();
    this.navBackBehavior.pop();
    console.log('popped style');
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
