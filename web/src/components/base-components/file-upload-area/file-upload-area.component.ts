import {
  Component,
  OnInit,
  EventEmitter,
  Input,
  Output,
  ElementRef,
  ViewChild,
  ChangeDetectorRef,
} from '@angular/core';
import uploadIcon from '@iconify/icons-carbon/upload';
import closeIcon from '@iconify/icons-carbon/close';
import { partial } from 'filesize';

export type UploadCallbackFunction = (
  files: File[],
  started: () => void,
  progress: (progress: number) => void,
  finished: (succeed: boolean) => void,
  aborted: () => void
) => void;

type uploadStatus = 'waiting' | 'uploading' | 'success' | 'failed';

@Component({
  selector: 'app-file-upload-area',
  templateUrl: './file-upload-area.component.html',
  styleUrls: ['./file-upload-area.component.less'],
})
export class FileUploadAreaComponent implements OnInit {
  constructor(private changeDetector: ChangeDetectorRef) {
    this.fileSizeGenerator = partial({ standard: 'iec' });
  }

  fileSizeGenerator: (size: number) => string;

  @Input() multiple: boolean = false;
  @Input() accept: string = '';
  @Input() large: boolean = false;
  @Input() uploadFunction: UploadCallbackFunction = undefined;

  private file_: FileList | undefined;

  set file(v: FileList | undefined) {
    this.file_ = v;
    this.fileChanged.emit(v);
    this.changeDetector.markForCheck();
  }
  get file() {
    return this.file_;
  }

  @Output() fileChanged = new EventEmitter<FileList>();

  isDraggingOver: boolean = false;
  progress: number = 0;
  status: uploadStatus = 'waiting';

  @ViewChild('fileUpload')
  fileUpload: ElementRef;

  get wrapperClass() {
    return { 'dragging-over': this.isDraggingOver };
  }

  get is_empty() {
    return this.file === undefined || this.file.length < 1;
  }

  get single_file() {
    return this.file[0];
  }

  get acceptsInfo() {
    let result: string;
    if (this.multiple) {
      result = '接受多个';
    } else {
      result = '只接受一个';
    }
    if (this.accept) {
      result += ` ${this.accept} 类型的`;
    }
    result += '文件';
    return result;
  }

  uploadIcon = uploadIcon;
  closeIcon = closeIcon;

  onClick() {
    this.fileUpload.nativeElement.click();
  }

  onDragOver(event: DragEvent) {
    this.isDraggingOver = true;
    event.stopPropagation();
    event.preventDefault();
  }

  onDragLeave(event: DragEvent) {
    this.isDraggingOver = false;
    event.stopPropagation();
    event.preventDefault();
  }

  onDrop(event: DragEvent) {
    this.isDraggingOver = false;
    let data = event.dataTransfer;
    if (
      !Array.from(data.files).every((file) =>
        this.appliesTo(file.name, this.accept)
      )
    ) {
      return;
    }
    this.file = data.files;
    this.fileUpload.nativeElement.files = data.files;
    event.preventDefault();
    event.stopPropagation();
  }

  onFileChanged(files: FileList) {
    this.file = files;
  }

  clearFile(event?: Event) {
    event?.stopPropagation();
    event?.preventDefault();
    this.file = undefined;
    this.fileUpload.nativeElement.value = '';
    this.status = 'waiting';
  }

  appliesTo(filename: string, accepts: string) {
    let req = accepts.split(',').map((x) => x.trim());
    return req.some((x) => filename.endsWith(x));
  }

  startUpload(event?: Event) {
    event?.stopPropagation();
    event?.preventDefault();
    if (this.uploadFunction === undefined) {
      console.warn('No upload function found');
      return;
    }
    this.uploadFunction(
      Array.from(this.file),
      () => {
        this.status = 'uploading';
      },
      (prog) => {
        this.progress = prog;
      },
      (success) => {
        if (success) {
          this.status = 'success';
        } else {
          this.status = 'failed';
        }
      },
      () => {
        this.status = 'waiting';
      }
    );
  }

  ngOnInit(): void {}
}
