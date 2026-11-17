# Requirements

What `wb` and its package shall do, one row per behaviour. Watchbill is a tool a team has to argue confidence in, so each row is tied to the tests that show it.

## Where they are

One file per area, `requirements/<area>.md`, named after the area in lower case. Each file holds one table.

## The form of an ID

`WB-<AREA>-<number>`, for example `WB-INIT-3`.

- `AREA` is two to eight capital letters, and names the command or the part: `INIT`, `CHECK`, `PKG`.
- The number starts at 1 within its area and has no leading zero.
- An ID is never used again and a row is never removed. A requirement that no longer applies is set to Deprecated and names the one that replaces it, if there is one.

As a pattern: `WB-[A-Z]{2,8}-[1-9][0-9]*`.

## The form of a row

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-XX-1 | `wb example` shall end with 2 when it is given an option it does not know. Issue: #0. | T | Draft |

The row above is an example and no requirement.

- **Requirement:** one "shall" per row, about one thing that can be seen from outside: what a command prints, writes, refuses or ends with. A row that needs "and" between two behaviours is two rows.
- **Cases:** a requirement that says "the same as" or "any" names its cases, so that its tests reach each of them. A requirement about a message gives its words, or says that any words do.
- **Issue:** a row names the issue that asked for it, as `Issue: #<number>`, at the end of its text.
- **Ver** is how the requirement is verified:

| Ver | Means |
|---|---|
| T | A test in `tests/` |
| I | Inspection: someone reads the code or the document |
| A | Analysis: a written argument |
| D | Demonstration: it is shown working, by hand |

- **Status:**

| Status | Means | Set by |
|---|---|---|
| Draft | Written and its ID reserved, not built | The session that writes the issue |
| Implemented | Built, with its verification in place | The session that builds the piece |
| Verified | The verification has been looked at and holds | A person, and no one else |
| Deprecated | No longer applies | A person, or the session whose issue says so |

## Tests carry the ID

- The name of a test starts with the ID of the requirement it covers, with `_` for `-`: `test_WB_INIT_2_a_file_that_is_there_is_left_as_it_is`.
- A test covers one requirement. A requirement may have several tests.
- A test that only names an ID does not cover it: the test has to fail when the requirement is not met.

## What has to hold

- Every row with Ver `T` and Status Implemented or Verified is named by at least one test.
- Every test names a row that exists.
- A Draft row names an issue.
- A session never sets Verified, and never moves a row back from Verified.
