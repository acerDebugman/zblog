# Single articles table with a status field, no draft revisions

An Article is one row with `status ∈ {draft, published}`. Publishing flips the status; editing a published article edits the live content immediately. We rejected the CMS-style model (a published version plus a separate draft revision that overwrites it on publish) because this is a single-author personal blog: the author accepts that mid-edit content on a published article is visible to visitors, and the revision model would double the storage/merge complexity for a scenario that rarely matters here.

**Consequences**: There is no way to edit a published article "offline" and republish atomically. If that need ever becomes real, this decision must be revisited — it is recorded here so nobody mistakes the simplicity for an oversight.
