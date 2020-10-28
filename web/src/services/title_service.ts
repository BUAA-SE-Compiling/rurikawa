import { Injectable } from '@angular/core';
import { Title } from '@angular/platform-browser';

@Injectable({ providedIn: 'root' })
export class TitleService {
  constructor(private title: Title) {
    this.titleStack.push(title.getTitle());
  }

  private titleStack: string[] = [];
  private sourceStack: string[] = [];

  public getTitle() {
    return this.title.getTitle();
  }

  public setTitle(s: string, source?: string) {
    if (source !== undefined) {
      if (
        this.sourceStack.length > 0 &&
        this.sourceStack[this.sourceStack.length - 1] === source
      ) {
        this.titleStack.pop();
      } else {
        this.titleStack.push(this.getTitle());
        this.sourceStack.push(source);
      }
    } else {
      this.titleStack.push(this.getTitle());
    }
    this.title.setTitle(s);
  }

  public revertTitle(source?: string) {
    if (
      source !== undefined &&
      this.sourceStack.length > 0 &&
      this.sourceStack[this.sourceStack.length - 1] === source
    ) {
      this.sourceStack.pop();
    }
    this.title.setTitle(this.titleStack.pop());
  }
}
