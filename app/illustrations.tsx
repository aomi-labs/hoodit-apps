/*
 * Hand-drawn riso-style illustrations for Hoodit.
 * Palette lives in globals.css; the SVGs read it through currentColor and CSS vars
 * so the same drawing can sit on cream, cobalt, or green plates.
 */

type SvgProps = { className?: string; title?: string };

const ink = "var(--ink)";
const cobalt = "var(--cobalt)";
const cobaltDeep = "var(--cobalt-2)";
const salmon = "var(--salmon)";
const blush = "var(--blush)";
const cream = "var(--paper)";
const green = "var(--green)";
const mint = "var(--mint)";
const mustard = "var(--mustard)";

/** Cat head, 120x120 coordinate space. Reused by CatMark and DeskScene. */
function CatHead() {
  return (
    <g strokeLinecap="round" strokeLinejoin="round">
      <circle cx="60" cy="60" r="55" fill={cobalt} stroke={ink} strokeWidth="5" />
      <path d="M60 30 C44 30 32 42 31 62 C30 80 44 92 60 92 C76 92 90 80 89 62 C88 42 76 30 60 30Z" fill={salmon} stroke={ink} strokeWidth="4" />
      <path d="M38 46 L33 22 L54 36Z" fill={salmon} stroke={ink} strokeWidth="4" />
      <path d="M82 46 L87 22 L66 36Z" fill={salmon} stroke={ink} strokeWidth="4" />
      <path d="M40 43 L38 30 L49 38Z" fill={cream} />
      <path d="M80 43 L82 30 L71 38Z" fill={cream} />
      <path d="M56 40 L55 47 M60 39 L60 47 M64 40 L65 47" stroke={ink} strokeWidth="2.5" />
      <path d="M38 60 Q47 52 56 60 Q47 67 38 60Z" fill={cream} stroke={ink} strokeWidth="3" />
      <path d="M64 60 Q73 52 82 60 Q73 67 64 60Z" fill={cream} stroke={ink} strokeWidth="3" />
      <circle cx="47.5" cy="61" r="3.6" fill={ink} />
      <circle cx="72.5" cy="61" r="3.6" fill={ink} />
      <path d="M37 60 Q47 50 57 60 L57 50 L37 50Z" fill={salmon} />
      <path d="M63 60 Q73 50 83 60 L83 50 L63 50Z" fill={salmon} />
      <path d="M40 59 Q47 53 55 59 M65 59 Q73 53 80 59" stroke={ink} strokeWidth="3" fill="none" />
      <ellipse cx="60" cy="76" rx="12" ry="7.5" fill={cream} />
      <path d="M56.5 71 L63.5 71 L60 75.5Z" fill={blush} stroke={ink} strokeWidth="2.5" />
      <path d="M60 75.5 L60 78.5 M54.5 79.5 Q60 84 60 78.5 Q60 84 65.5 79.5" stroke={ink} strokeWidth="2.5" fill="none" />
      <path d="M22 66 L42 70 M23 75 L42 75 M98 66 L78 70 M97 75 L78 75" stroke={ink} strokeWidth="2.5" />
      <circle cx="41" cy="78" r="3.5" fill={blush} opacity=".8" />
      <circle cx="79" cy="78" r="3.5" fill={blush} opacity=".8" />
    </g>
  );
}

/** Small round avatar: cat head only. Used for nav, footer, widget avatar, QR center. */
export function CatMark({ className, title = "Hoodit" }: SvgProps) {
  return (
    <svg className={className} viewBox="0 0 120 120" role="img" aria-label={title}>
      <CatHead />
    </svg>
  );
}

/** Hooded cat bust with crossed paws. The hero mascot. */
export function HooditCat({ className, title = "Hoodit, a cat wearing a hood" }: SvgProps) {
  return (
    <svg className={className} viewBox="0 0 240 240" role="img" aria-label={title}>
      <g strokeLinecap="round" strokeLinejoin="round">
        {/* hood */}
        <path d="M120 14 C88 16 58 44 50 84 C42 124 30 160 22 226 L218 226 C210 160 198 124 190 84 C182 44 152 16 120 14Z" fill={cobalt} stroke={ink} strokeWidth="6" />
        <path d="M120 24 C112 40 108 56 110 70 M86 42 C76 60 72 76 74 90 M154 42 C164 60 168 76 166 90 M44 160 C52 176 58 196 60 222 M196 160 C188 176 182 196 180 222" stroke={ink} strokeWidth="4" fill="none" />
        {/* hood rim */}
        <path d="M120 60 C86 60 62 82 60 118 C58 152 84 176 120 176 C156 176 182 152 180 118 C178 82 154 60 120 60Z" fill={cobaltDeep} stroke={ink} strokeWidth="6" />
        {/* ears poking through */}
        <path d="M80 86 L70 42 L104 70Z" fill={salmon} stroke={ink} strokeWidth="5" />
        <path d="M160 86 L170 42 L136 70Z" fill={salmon} stroke={ink} strokeWidth="5" />
        <path d="M82 80 L76 54 L96 72Z" fill={cream} />
        <path d="M158 80 L164 54 L144 72Z" fill={cream} />
        {/* face */}
        <path d="M120 68 C92 68 70 86 68 118 C66 146 88 168 120 168 C152 168 174 146 172 118 C170 86 148 68 120 68Z" fill={salmon} stroke={ink} strokeWidth="5" />
        <path d="M112 84 L110 97 M120 82 L120 97 M128 84 L130 97 M100 92 L103 102 M140 92 L137 102" stroke={ink} strokeWidth="3" />
        {/* eyes */}
        <path d="M82 118 Q100 104 118 118 Q100 130 82 118Z" fill={cream} stroke={ink} strokeWidth="4" />
        <path d="M122 118 Q140 104 158 118 Q140 130 122 118Z" fill={cream} stroke={ink} strokeWidth="4" />
        <ellipse cx="100" cy="119" rx="6.5" ry="7.5" fill={ink} />
        <ellipse cx="140" cy="119" rx="6.5" ry="7.5" fill={ink} />
        <path d="M80 118 Q100 100 120 118 L120 100 L80 100Z" fill={salmon} />
        <path d="M120 118 Q140 100 160 118 L160 100 L120 100Z" fill={salmon} />
        <path d="M84 116 Q100 104 116 116 M124 116 Q140 104 156 116" stroke={ink} strokeWidth="4" fill="none" />
        {/* muzzle */}
        <ellipse cx="120" cy="146" rx="24" ry="14" fill={cream} />
        <path d="M113 137 L127 137 L120 145Z" fill={blush} stroke={ink} strokeWidth="3" />
        <path d="M120 145 L120 150 M109 152 Q120 160 120 150 Q120 160 131 152" stroke={ink} strokeWidth="3" fill="none" />
        <path d="M56 132 L92 138 M54 145 L92 145 M58 158 L92 152 M184 132 L148 138 M186 145 L148 145 M182 158 L148 152" stroke={ink} strokeWidth="3" />
        <circle cx="86" cy="150" r="6" fill={blush} opacity=".75" />
        <circle cx="154" cy="150" r="6" fill={blush} opacity=".75" />
        {/* crossed paws */}
        <path d="M60 228 C66 206 84 194 112 194 L150 196 C168 198 178 212 180 228Z" fill={cobalt} stroke={ink} strokeWidth="5" />
        <path d="M72 228 C78 212 92 204 112 204 C134 204 148 212 152 228Z" fill={salmon} stroke={ink} strokeWidth="5" />
        <path d="M92 214 L94 228 M108 210 L110 228 M124 212 L126 228" stroke={ink} strokeWidth="3" />
        <path d="M40 228 L200 228" stroke={ink} strokeWidth="6" />
      </g>
    </svg>
  );
}

/** Pile of bills with a coin. Fund step. */
export function MoneyScene({ className, title = "A pile of bills" }: SvgProps) {
  const bill = (x: number, y: number, r: number, key: string) => (
    <g key={key} transform={`translate(${x} ${y}) rotate(${r})`} strokeLinecap="round" strokeLinejoin="round">
      <rect x="-46" y="-26" width="92" height="52" rx="3" fill={mint} stroke={ink} strokeWidth="4" />
      <rect x="-38" y="-18" width="76" height="36" rx="2" fill="none" stroke={green} strokeWidth="2" />
      <path d="M-30 -8 C-22 -14 -14 -2 -6 -8 S10 -14 18 -8 M-30 2 C-20 -4 -10 8 0 2 S16 -4 26 2 M-30 12 C-22 6 -12 18 -2 12 S12 6 22 12" stroke={green} strokeWidth="2.5" fill="none" />
      <circle cx="0" cy="0" r="9" fill={mint} stroke={green} strokeWidth="2.5" />
      <path d="M0 -6 L0 6 M-3 -3 Q0 -6 3 -3 Q0 0 -3 3 Q0 6 3 3" stroke={green} strokeWidth="2" fill="none" />
    </g>
  );
  return (
    <svg className={className} viewBox="0 0 260 170" role="img" aria-label={title}>
      {bill(70, 118, -14, "a")}
      {bill(190, 112, 9, "b")}
      {bill(126, 82, -4, "c")}
      {bill(116, 52, 6, "d")}
      <g transform="translate(222 46)" strokeLinejoin="round">
        <circle r="22" fill={mustard} stroke={ink} strokeWidth="4" />
        <path d="M0 -13 L0 13 M-7 -6 Q0 -12 7 -6 Q0 0 -7 6 Q0 12 7 6" stroke={ink} strokeWidth="3.5" fill="none" strokeLinecap="round" />
      </g>
      <g transform="translate(30 42)" strokeLinejoin="round">
        <circle r="15" fill={mustard} stroke={ink} strokeWidth="3.5" />
        <path d="M0 -9 L0 9 M-5 -4 Q0 -8 5 -4 Q0 0 -5 4 Q0 8 5 4" stroke={ink} strokeWidth="3" fill="none" strokeLinecap="round" />
      </g>
    </svg>
  );
}

/** Bar columns with an up arrow and coins. The edge section. */
export function ChartScene({ className, title = "A rising chart" }: SvgProps) {
  const col = (x: number, h: number, key: string) => (
    <g key={key} strokeLinejoin="round">
      <path d={`M${x} ${190 - h} L${x + 12} ${182 - h} L${x + 42} ${182 - h} L${x + 42} 182 L${x + 30} 190 L${x} 190Z`} fill={cobalt} stroke={ink} strokeWidth="4" />
      <path d={`M${x + 30} 190 L${x + 30} ${190 - h} L${x} ${190 - h}`} fill="none" stroke={ink} strokeWidth="4" />
      <rect x={x + 6} y={196 - h} width="18" height={h - 6} fill={green} />
      <path d={`M${x + 30} ${190 - h} L${x + 42} ${182 - h}`} stroke={ink} strokeWidth="4" />
    </g>
  );
  return (
    <svg className={className} viewBox="0 0 300 200" role="img" aria-label={title}>
      {col(24, 60, "1")}
      {col(84, 92, "2")}
      {col(144, 76, "3")}
      {col(204, 130, "4")}
      <path d="M18 132 L70 92 L104 116 L176 46" fill="none" stroke={green} strokeWidth="14" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M150 34 L194 28 L188 72Z" fill={green} stroke={green} strokeWidth="6" strokeLinejoin="round" />
      <g transform="translate(262 40)" strokeLinejoin="round">
        <circle r="18" fill={mustard} stroke={ink} strokeWidth="3.5" />
        <path d="M0 -11 L0 11 M-6 -5 Q0 -10 6 -5 Q0 0 -6 5 Q0 10 6 5" stroke={ink} strokeWidth="3" fill="none" strokeLinecap="round" />
      </g>
      <g transform="translate(232 108) rotate(-18)" strokeLinejoin="round">
        <ellipse rx="16" ry="10" fill={mustard} stroke={ink} strokeWidth="3" />
      </g>
      <g transform="translate(118 20) rotate(14)" strokeLinejoin="round">
        <ellipse rx="14" ry="9" fill={mustard} stroke={ink} strokeWidth="3" />
      </g>
    </svg>
  );
}

/** Cat at a laptop. Demo section. */
export function DeskScene({ className, title = "Hoodit typing at a laptop" }: SvgProps) {
  return (
    <svg className={className} viewBox="0 0 260 190" role="img" aria-label={title}>
      <g transform="translate(70 -14)">
        <CatHead />
      </g>
      <g strokeLinecap="round" strokeLinejoin="round">
        {/* shoulders */}
        <path d="M56 138 C70 112 96 104 130 104 C164 104 190 112 204 138Z" fill={cobalt} stroke={ink} strokeWidth="4" />
        {/* laptop lid */}
        <path d="M96 78 L210 70 L216 136 L102 144Z" fill={cobalt} stroke={ink} strokeWidth="4" />
        <path d="M106 86 L200 80 L205 128 L112 134Z" fill={cream} />
        <path d="M116 96 L160 93 M116 106 L188 101 M116 116 L176 112 M116 126 L150 124" stroke={cobalt} strokeWidth="3" />
        <path d="M168 110 L192 108 L192 122 L168 124Z" fill={salmon} />
        {/* laptop base */}
        <path d="M74 152 L224 142 L244 168 L80 178Z" fill={cream} stroke={ink} strokeWidth="4" />
        <path d="M92 156 L214 148 L226 162 L96 170Z" fill="none" stroke={ink} strokeWidth="2.5" />
        <path d="M110 154 L118 166 M132 153 L140 165 M154 151 L162 163 M176 150 L184 162 M198 149 L206 161" stroke={ink} strokeWidth="2" />
        {/* paws */}
        <path d="M84 150 C88 142 100 140 108 146 L112 158 L90 160Z" fill={salmon} stroke={ink} strokeWidth="3.5" />
        <path d="M152 146 C158 138 170 138 176 144 L178 156 L156 158Z" fill={salmon} stroke={ink} strokeWidth="3.5" />
        <path d="M94 150 L96 158 M100 148 L102 157 M162 146 L164 155 M168 145 L170 154" stroke={ink} strokeWidth="2.5" />
        {/* mug */}
        <path d="M22 148 L52 148 L48 180 L26 180Z" fill={salmon} stroke={ink} strokeWidth="3.5" />
        <path d="M52 154 C62 152 64 168 50 170" fill="none" stroke={ink} strokeWidth="3.5" />
        <path d="M30 136 C34 128 30 126 34 118 M42 136 C46 128 42 126 46 118" fill="none" stroke={ink} strokeWidth="2.5" opacity=".5" />
      </g>
    </svg>
  );
}
