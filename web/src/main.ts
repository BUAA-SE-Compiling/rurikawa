import { enableProdMode } from '@angular/core';
import { platformBrowserDynamic } from '@angular/platform-browser-dynamic';

import { AppModule } from './app/app.module';
import { environment } from './environments/environment';
import dayjs from 'dayjs';
import * as utc from 'dayjs/plugin/utc';
import { startWatchingDarkMode } from './services/dark_mode_watcher';

dayjs.extend(utc);

if (environment.production) {
  enableProdMode();
}

platformBrowserDynamic()
  .bootstrapModule(AppModule)
  .catch((err) => console.error(err));

startWatchingDarkMode();
