export function Crystal({ small = false }: { small?: boolean }) {
  return (
    <svg
      className={small ? "crystal small" : "crystal"}
      viewBox="0 0 80 90"
      fill="none"
      aria-label="Cristal Cryon"
      role="img"
    >
      <defs>
        <linearGradient id="ice" x2="1" y2="1">
          <stop stopColor="#68efff" />
          <stop offset=".5" stopColor="#168bff" />
          <stop offset="1" stopColor="#134cbb" />
        </linearGradient>
      </defs>
      <path
        d="M40 3 73 23v43L40 87 7 66V23Z"
        fill="url(#ice)"
        stroke="#66eaff"
        strokeWidth="2"
      />
      <path
        d="m7 23 33 20 33-20M40 43v44M40 3v40L7 66m33-23 33 23"
        stroke="#b7f6ff"
        strokeOpacity=".45"
      />
      <path d="m40 3 33 20-33 20Z" fill="#86f0ff" fillOpacity=".2" />
    </svg>
  );
}
