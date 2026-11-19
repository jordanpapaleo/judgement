import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

/**
 * Markdown with raw HTML skipped and only safe link schemes (react-markdown's
 * default URL transform), plus GitHub tables. Links never navigate the window
 * — the app makes no network calls — so they render as inert text.
 */
export function Markdown({
  text,
  variant = "guidance",
}: {
  text: string;
  variant?: "guidance" | "document";
}) {
  return (
    <div className={`selectable ${variant === "document" ? "prose-document" : "prose-guidance"}`}>
      <ReactMarkdown
        skipHtml
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ children, href }) => (
            <span className="underline decoration-dotted" title={href ?? undefined}>
              {children}
            </span>
          ),
          img: ({ alt }) => <span>{alt}</span>,
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
