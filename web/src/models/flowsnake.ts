import dayjs, { Dayjs } from 'dayjs';

const charToBase32 = [
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  0,
  1,
  2,
  3,
  4,
  5,
  6,
  7,
  8,
  9,
  255,
  255,
  255,
  255,
  255,
  255,
  255,
  10,
  11,
  12,
  13,
  14,
  15,
  16,
  17,
  255,
  18,
  19,
  255,
  20,
  21,
  255,
  22,
  23,
  24,
  25,
  26,
  255,
  27,
  28,
  29,
  30,
  31,
  255,
  255,
  255,
  255,
  255,
  255,
  10,
  11,
  12,
  13,
  14,
  15,
  16,
  17,
  255,
  18,
  19,
  255,
  20,
  21,
  255,
  22,
  23,
  24,
  25,
  26,
  255,
  27,
  28,
  29,
  30,
  31,
  255,
  255,
  255,
  255,
  255,
];

export function extractTime(flowsnake: string): Dayjs {
  if (flowsnake.length !== 13 && flowsnake.length !== 14) {
    throw new Error(
      `Not a flowsnake value. Invalid length ${flowsnake.length}, expected 13`
    );
  }

  let timestamp = 0;
  for (let i = 0; i < 7; i++) {
    let charcode = flowsnake.charCodeAt(i);
    if (charcode > 128) {
      throw new Error(
        `Not a flowsnake value. Invalid char at ${i}: '${String.fromCharCode(
          charcode
        )}'`
      );
      11;
    }
    let base32 = charToBase32[charcode];
    // timestamp is 42 bits long, so no bitwise operations here
    timestamp = timestamp * 32 + base32;
  }
  return dayjs.unix(timestamp);
}

export const FLOWSNAKE_MAX = '7ffffffffffff';
export const FLOWSNAKE_MIN = '0000000000000';
