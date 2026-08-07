# zblog

A single-author personal blog: an admin backend for writing articles, and a public frontend for reading them.

## Language

**Author**:
The single person who writes and manages articles. There is exactly one; articles carry no author dimension.
_Avoid_: user, account, writer

**Article**:
The single content entity of the blog, written in Markdown. Every article has exactly one status.
_Avoid_: post, entry, blog

**Draft**:
An Article whose status is `draft` — unpublished work in progress, never visible to Visitors. Not a separate entity.
_Avoid_: revision, version, autosave

**Published**:
An Article whose status is `published` — visible to Visitors. Editing a published article edits the live content directly.

**Visitor**:
An anonymous reader of the public frontend. Visitors never see Drafts.
_Avoid_: guest, reader, user

**Slug**:
The unique, URL-friendly identifier of an Article. The public URL of a published article is `/posts/{slug}`.
_Avoid_: permalink, alias

**Math Expression**:
A formula written inside an Article's Markdown source: `$...$` for inline, `$$...$$` for display blocks.
_Avoid_: equation block, latex

**Live Preview**:
The in-editor real-time rendering of the Markdown being written, shown next to the editor in the admin.
_Avoid_: WYSIWYG

**Draft Preview**:
A protected route (`/preview/{slug}`) rendering a Draft in the real site layout, for the Author to verify appearance before publishing.
_Avoid_: staging page

**PageView**:
One recorded visit to a public page, capturing timestamp, path, Article (if any), raw IP address, user agent, and referer. The single source for all traffic statistics.
_Avoid_: hit, log entry, event
