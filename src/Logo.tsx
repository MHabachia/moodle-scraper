export function Logo({size = 40}: {size?: number}) {
  return (
    <svg width={size} height={size} viewBox="0 0 512 512" aria-label="Moodle Scraper" role="img">
      <defs>
        <linearGradient id="ms-g" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#818CF8" />
          <stop offset="1" stopColor="#4338CA" />
        </linearGradient>
      </defs>
      <rect width="512" height="512" rx="112" fill="url(#ms-g)" />
      <path d="M256 88 L420 162 L256 236 L92 162 Z" fill="#fff" />
      <path d="M164 202 v64 c0 28 42 50 92 50 s92-22 92-50 v-64 l-92 42 z" fill="#fff" opacity=".88" />
      <path d="M420 162 v66" stroke="#fff" strokeWidth="12" strokeLinecap="round" />
      <path d="M256 334 V398 M214 362 L256 404 L298 362 M170 440 H342" stroke="#fff" strokeWidth="22" strokeLinecap="round" strokeLinejoin="round" fill="none" />
    </svg>
  );
}
