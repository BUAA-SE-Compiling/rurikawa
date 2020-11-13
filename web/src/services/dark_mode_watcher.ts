import { Injectable } from '@angular/core';

export function startWatchingDarkMode() {
  if (
    window.matchMedia &&
    window.matchMedia('(prefers-color-scheme: dark)').matches
  ) {
    onDarkMode();
  } else {
    onLightMode();
  }
  window
    .matchMedia('(prefers-color-scheme: dark)')
    .addEventListener('change', (event) => {
      if (event.matches) {
        onDarkMode();
      } else {
        onLightMode();
      }
    });
}

export let darkModeWatcherInstance: DarkModeWatcher;

@Injectable({ providedIn: 'root' })
export class DarkModeWatcher {
  constructor() {
    darkModeWatcherInstance = this;
  }
}

const lightModeAssets = {
  appleTouchIcon: 'assets/apple-touch-icon.png',
  icon32: 'assets/favicon-32x32.png',
  icon16: 'assets/favicon-16x16.png',
  iconIco: 'assets/favicon.ico',
};

const darkModeAssets = {
  appleTouchIcon: 'assets/darkmode/apple-touch-icon.png',
  icon32: 'assets/darkmode/favicon-32x32.png',
  icon16: 'assets/darkmode/favicon-16x16.png',
  iconIco: 'assets/darkmode/favicon.ico',
};

function onLightMode() {
  try {
    document.getElementsByTagName('body')[0].classList.add('default-palette');
    document
      .getElementsByTagName('body')[0]
      .classList.remove('inverted-palette');
    (document.getElementById('apple-touch-icon') as HTMLLinkElement).href =
      lightModeAssets.appleTouchIcon;
    (document.getElementById('icon-32') as HTMLLinkElement).href =
      lightModeAssets['icon32'];
    (document.getElementById('icon-16') as HTMLLinkElement).href =
      lightModeAssets['icon16'];
    (document.getElementById('icon-ico') as HTMLLinkElement).href =
      lightModeAssets['iconIco'];
  } catch (e) {
    console.error(e);
  }
}

function onDarkMode() {
  try {
    document.body.classList.remove('default-palette');
    document.body.classList.add('inverted-palette');
    (document.getElementById('apple-touch-icon') as HTMLLinkElement).href =
      darkModeAssets.appleTouchIcon;
    (document.getElementById('icon-32') as HTMLLinkElement).href =
      darkModeAssets['icon32'];
    (document.getElementById('icon-16') as HTMLLinkElement).href =
      darkModeAssets['icon16'];
    (document.getElementById('icon-ico') as HTMLLinkElement).href =
      darkModeAssets['iconIco'];
  } catch (e) {
    console.error(e);
  }
}
