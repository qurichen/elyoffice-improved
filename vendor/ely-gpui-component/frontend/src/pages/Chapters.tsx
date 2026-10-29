import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { pad, search } from "../data";
import { Find } from "../Find";

export function Chapters() {
  const [query, setQuery] = useState("");
  const navigate = useNavigate();
  const found = search(query);
  useEffect(() => {
    document.title = "Components · Ely";
  }, []);
  const first = () => found[0] && navigate({ to: "/components/$page/$story/", params: { page: found[0].slug, story: found[0].stories[0].slug } });
  return (
    <section className="chapters">
      <h1>Components.</h1>
      <Find className="find" query={query} onQuery={setQuery} onEnter={first} />
      {found.length === 0 && <p className="none">Nothing matches “{query}”.</p>}
      <ol>
        {found.map((c) => (
          <li key={c.slug}>
            <span className="n">{pad(c.number)}</span>
            <Link className="name" to="/components/$page/$story/" params={{ page: c.slug, story: c.stories[0].slug }}>
              {c.title}
            </Link>
            <span className="titles">
              {c.stories.map((s) => (
                <Link key={s.slug} to="/components/$page/$story/" params={{ page: c.slug, story: s.slug }}>
                  {s.name}
                </Link>
              ))}
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}
