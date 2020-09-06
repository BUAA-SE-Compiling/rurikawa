import { Component, OnInit } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { JobItem } from 'src/models/job-items';

@Component({
  selector: 'app-test-suite-view',
  templateUrl: './test-suite-view.component.html',
  styleUrls: ['./test-suite-view.component.styl'],
})
export class TestSuiteViewComponent implements OnInit {
  constructor(private route: ActivatedRoute) {}

  id: string;

  items: JobItem[] = [
    {
      id: '12qwp43e34mc8',
      totalItem: 15,
      finishedItem: 8,
      status: [{ status: 'ac', cnt: 16 }],
      time: new Date('1970-1-1 12:00'),
    },
    {
      id: '12qw3ev8i9sdz',
      totalItem: 15,
      finishedItem: 8,
      status: [
        { status: 'ac', cnt: 7 },
        { status: 'wa', cnt: 6 },
        { status: 'nt', cnt: 3 },
      ],
      time: new Date('1970-1-1 12:00'),
    },
  ];

  ngOnInit(): void {
    this.route.paramMap.subscribe({
      next: (map) => {
        this.id = map.get('id');
      },
    });
  }
}
