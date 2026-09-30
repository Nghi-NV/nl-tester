export interface IdeRelease {
  version: string;
  name: string;
  notes: string;
  url: string;
}

export interface IdeReleaseInfo {
  latest: IdeRelease;
}

interface GitHubRelease {
  tag_name: string;
  name: string | null;
  body: string | null;
  html_url: string;
  draft: boolean;
  prerelease: boolean;
}

const RELEASES_URL = 'https://api.github.com/repos/Nghi-NV/nl-tester/releases?per_page=100';

function versionParts(version: string): number[] {
  return version.split('.').map(part => Number.parseInt(part, 10));
}

export function compareVersions(left: string, right: string): number {
  const a = versionParts(left);
  const b = versionParts(right);
  for (let index = 0; index < Math.max(a.length, b.length); index += 1) {
    const difference = (a[index] ?? 0) - (b[index] ?? 0);
    if (difference !== 0) return difference;
  }
  return 0;
}

export async function fetchLatestIdeRelease(): Promise<IdeReleaseInfo> {
  const response = await fetch(RELEASES_URL, {
    headers: { Accept: 'application/vnd.github+json' },
  });
  if (!response.ok) {
    throw new Error(response.status === 403
      ? 'GitHub API rate limit reached. Try again later.'
      : `GitHub returned ${response.status} while checking for updates.`);
  }

  const releases = await response.json() as GitHubRelease[];
  const ideReleases = releases.flatMap(release => {
    const match = /^lumi-ide-v(\d+\.\d+\.\d+)$/.exec(release.tag_name);
    if (!match || release.draft || release.prerelease) return [];
    return [{
      version: match[1],
      name: release.name || `Lumi IDE ${match[1]}`,
      notes: release.body?.trim() || 'No release notes were provided.',
      url: release.html_url,
    } satisfies IdeRelease];
  }).sort((left, right) => compareVersions(right.version, left.version));

  const latest = ideReleases[0];
  if (!latest) throw new Error('No published Lumi IDE releases were found.');
  return { latest };
}
