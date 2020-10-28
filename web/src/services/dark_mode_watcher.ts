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
function onLightMode() {
  document.getElementsByTagName('body')[0].classList.add('default-palette');
  document.getElementsByTagName('body')[0].classList.remove('inverted-palette');
}

function onDarkMode() {
  document.getElementsByTagName('body')[0].classList.remove('default-palette');
  document.getElementsByTagName('body')[0].classList.add('inverted-palette');
}
