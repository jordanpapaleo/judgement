// The analyzer command is stored and executed as an argument array, never
// through a shell. These convert between that array and the one-line text
// field in Settings: quotes group words; nothing is expanded.

export function splitArgs(text: string): string[] {
  const args: string[] = [];
  let current = "";
  let quote: '"' | "'" | null = null;
  let started = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === quote) quote = null;
      else if (c === "\\" && quote === '"' && i + 1 < text.length) current += text[++i];
      else current += c;
    } else if (c === '"' || c === "'") {
      quote = c;
      started = true;
    } else if (c === "\\" && i + 1 < text.length) {
      current += text[++i];
      started = true;
    } else if (/\s/.test(c)) {
      if (started) args.push(current);
      current = "";
      started = false;
    } else {
      current += c;
      started = true;
    }
  }
  if (quote) throw new Error("The command has an unclosed quote.");
  if (started) args.push(current);
  return args;
}

export function joinArgs(args: string[]): string {
  return args
    .map((a) => (a === "" || /[\s"'\\]/.test(a) ? `'${a.replaceAll("'", `'"'"'`)}'` : a))
    .join(" ");
}
