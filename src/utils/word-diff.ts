export interface WordReplacement {
  original: string;
  correct: string;
}

function tokenize(text: string): string[] {
  return text.match(/[\p{L}\p{N}]+|[^\s\p{L}\p{N}]+|\s+/gu) ?? [];
}

function lcsLength(a: string[], b: string[]): number[][] {
  const m = a.length;
  const n = b.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (a[i - 1] === b[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }
  return dp;
}

function flush(removed: string[], added: string[], out: WordReplacement[]) {
  const orig = removed.join("").trim();
  const corr = added.join("").trim();
  if (orig && corr && orig !== corr) {
    out.push({ original: orig, correct: corr });
  }
  removed.length = 0;
  added.length = 0;
}

export function detectReplacements(original: string, corrected: string): WordReplacement[] {
  if (original === corrected) return [];

  const tokensA = tokenize(original);
  const tokensB = tokenize(corrected);
  const dp = lcsLength(tokensA, tokensB);

  const replacements: WordReplacement[] = [];
  const removed: string[] = [];
  const added: string[] = [];

  let i = tokensA.length;
  let j = tokensB.length;
  while (i > 0 && j > 0) {
    if (tokensA[i - 1] === tokensB[j - 1]) {
      if (removed.length > 0 && added.length > 0) {
        flush(removed, added, replacements);
      }
      i--;
      j--;
    } else if (dp[i - 1][j] >= dp[i][j - 1]) {
      removed.unshift(tokensA[i - 1]);
      i--;
    } else {
      added.unshift(tokensB[j - 1]);
      j--;
    }
  }
  while (i > 0) {
    removed.unshift(tokensA[i - 1]);
    i--;
  }
  while (j > 0) {
    added.unshift(tokensB[j - 1]);
    j--;
  }

  if (removed.length > 0 && added.length > 0) {
    flush(removed, added, replacements);
  }

  return replacements;
}
