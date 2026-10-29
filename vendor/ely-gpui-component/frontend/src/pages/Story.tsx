import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { Live } from "../Live";
import { Capture, Windows, isCaptured } from "../Native";
import { all, chapters, search } from "../data";
import { Find, typing } from "../Find";
import { NotFound } from "./NotFound";

export function Story() {
  const { page, story } = useParams({ strict: false });
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const chapter = chapters.find((c) => c.slug === page);
  const found = chapter?.stories.find((s) => s.slug === story);
  const at = all.findIndex((e) => e.story === found);
  const [prev, next] = at < 0 ? [] : [all[at - 1], all[at + 1]];

  useEffect(() => {
    if (chapter && found) document.title = `${found.name} · ${chapter.title} · Ely`;
  }, [chapter, found]);

  useEffect(() => {
    document.querySelector('.side li [aria-current="page"]')?.scrollIntoView({ block: "nearest" });
  }, [page, story]);

  useEffect(() => {
    const step = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey || typing(e)) return;
      const to = e.key === "ArrowLeft" ? prev : e.key === "ArrowRight" ? next : undefined;
      if (to) navigate({ to: "/components/$page/$story/", params: { page: to.chapter.slug, story: to.story.slug } });
    };
    addEventListener("keydown", step);
    return () => removeEventListener("keydown", step);
  }, [prev, next, navigate]);

  if (!chapter || !found) return <NotFound what={`No story at ${page}/${story}.`} />;
  const shown = search(query);
  const first = () => shown[0] && navigate({ to: "/components/$page/$story/", params: { page: shown[0].slug, story: shown[0].stories[0].slug } });
  return (
    <div className="reader">
      <aside className="side" data-lenis-prevent>
        <Find className="find small" query={query} onQuery={setQuery} onEnter={first} />
        <nav aria-label="Chapters">
          {shown.map((c) => (
            <div key={c.slug}>
              <Link className="side-chapter" to="/components/$page/$story/" params={{ page: c.slug, story: c.stories[0].slug }} data-on={c.slug === chapter.slug || undefined}>
                {c.title}
              </Link>
              {(query || c.slug === chapter.slug) && (
                <ol>
                  {c.stories.map((s) => (
                    <li key={s.slug}>
                      <Link to="/components/$page/$story/" params={{ page: c.slug, story: s.slug }} activeProps={{ "aria-current": "page" }}>
                        {s.name}
                      </Link>
                    </li>
                  ))}
                </ol>
              )}
            </div>
          ))}
        </nav>
      </aside>
      <article className="story">
        <Link to="/components/" className="crumb">
          {chapter.title}
        </Link>
        <h1>{found.name}</h1>
        {isCaptured(chapter.slug, found.slug) ? (
          <Capture page={chapter.slug} story={found.slug} title={found.name} />
        ) : (
          <>
            <Live key={`${chapter.slug}/${found.slug}`} page={chapter.slug} story={found.slug} title={found.name} />
            <Windows page={chapter.slug} story={found.slug} title={found.name} />
          </>
        )}
        <nav className="pager" aria-label="More stories">
          {prev && (
            <Link to="/components/$page/$story/" params={{ page: prev.chapter.slug, story: prev.story.slug }}>
              ← {prev.story.name}
            </Link>
          )}
          {next && (
            <Link className="next" to="/components/$page/$story/" params={{ page: next.chapter.slug, story: next.story.slug }}>
              {next.story.name} →
            </Link>
          )}
        </nav>
      </article>
    </div>
  );
}
