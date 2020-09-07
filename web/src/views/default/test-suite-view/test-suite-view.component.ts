import { Component, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { JobItem } from 'src/models/job-items';
import repo from '@iconify/icons-carbon/link';
import branch from '@iconify/icons-carbon/branch';

@Component({
  selector: 'app-test-suite-view',
  templateUrl: './test-suite-view.component.html',
  styleUrls: ['./test-suite-view.component.styl'],
})
export class TestSuiteViewComponent implements OnInit {
  constructor(private route: ActivatedRoute, private router: Router) {}

  readonly repo = repo;
  readonly branch = branch;

  id: string;

  items: JobItem[] = [
    {
      id: '12qwp43e34mc8',
      totalItem: 15,
      finishedItem: 8,
      status: [{ status: 'Accepted', cnt: 16 }],
      time: new Date('1970-1-1 12:00'),
    },
    {
      id: '12qw3ev8i9sdz',
      totalItem: 15,
      finishedItem: 8,
      status: [
        { status: 'Accepted', cnt: 7 },
        { status: 'WrongAnswer', cnt: 6 },
        { status: 'NotRunned', cnt: 3 },
      ],
      time: new Date('1970-1-1 12:00'),
    },
  ];

  gotoJob(id: string) {
    this.router.navigate(['job', id]);
  }

  ngOnInit(): void {
    this.route.paramMap.subscribe({
      next: (map) => {
        this.id = map.get('id');
      },
    });
  }
}
