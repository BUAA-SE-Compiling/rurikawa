import { reduce } from 'lodash';
import { Job } from 'src/models/job-items';
import { TestCaseDefinition, TestSuite } from 'src/models/server-types';

export function resultBriefMain(
  job: Job,
  testSuite: TestSuite | undefined,
  numberFormatter: Intl.NumberFormat
) {
  if (!job) {
    return 'Loading';
  }
  if (job.stage !== 'Finished') {
    return job.stage;
  }
  if (job.resultKind !== 'Accepted') {
    return job.resultKind;
  }

  if (testSuite?.scoringMode === 'Floating') {
    let acScore = 0;

    // tslint:disable-next-line: forin
    for (let idx in job.results) {
      let res = job.results[idx];
      if (res.kind === 'Accepted') {
        acScore += res.score ?? this.flatCaseMap.get(idx)?.baseScore ?? 1.0;
      }
    }
    return numberFormatter.format(acScore);
  } else {
    let acCnt = 0;

    // tslint:disable-next-line: forin
    for (let idx in job.results) {
      let res = job.results[idx];
      if (res.kind === 'Accepted') {
        acCnt++;
      }
    }

    return acCnt.toString();
  }
}

export function resultBriefSub(
  job: Job,
  testSuite: TestSuite | undefined,
  numberFormatter: Intl.NumberFormat
) {
  if (!job || job.stage != 'Finished' || job.resultKind != 'Accepted')
    return '';
  if (testSuite?.scoringMode === 'Floating') {
    let res = 0;
    for (let key in testSuite.testGroups) {
      for (let val of testSuite.testGroups[key]) {
        res += val.baseScore ?? 1;
      }
    }
    return '/' + numberFormatter.format(res);
  } else {
    let res = reduce(job.results, (p) => p + 1, 0);
    return '/' + numberFormatter.format(res);
  }
}
