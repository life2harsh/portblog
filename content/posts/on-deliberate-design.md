# On Deliberate Design

Most developer tools look the way they do because no one decided they should look different.

That sounds harsh, but it is mostly accurate. A web framework ships a documentation site using the same components it teaches. A CLI tool uses the default terminal output because the alternative — spending a week on color codes, column alignment, and signal handling — is hard to justify in a project roadmap.

The result is a world of tools that function but do not *feel* considered. They pass information. They do not create an experience.

## What deliberate design actually means

Deliberate design is not aesthetics for its own sake. It is the decision to notice every surface a user touches and ask: *does this serve them?*

For a terminal tool, that might mean:

- Aligning output into scannable columns instead of dumping key: value pairs.
- Using color as *signal*, not decoration — errors red, warnings amber, success quiet.
- Obeying `NO_COLOR`, `TERM`, and `isatty` checks.
- Writing error messages in plain prose, not exception stack traces.

None of these are difficult in isolation. The difficulty is remembering to care about all of them at once.

## The 5% problem

Products succeed or fail on 95% of use cases. The 5% at the edges — slow networks, unusual locales, accessibility needs, non-default configurations — are easy to defer forever. Users with those needs learn to work around your tool's rough edges, because the core value is still there.

But the edges are where trust is built or lost. A tool that handles the 5% is one that someone recommends without caveats.

## Practical suggestions

**Write output for humans first, machines second.** If you need both, add `--json` or `--porcelain`.

**Treat your error path as carefully as your happy path.** Users encounter errors more than they encounter success.

**Resist the urge to add color to everything.** Syntax highlighting in a streaming log makes it harder to parse, not easier.

**Test your tool in a 80-column terminal with a default color scheme.** Most users have not configured their environment to match your screenshots.

---

These are obvious things. The hard part is doing them every time, for every tool, without being asked.
