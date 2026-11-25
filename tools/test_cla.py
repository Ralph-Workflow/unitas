from __future__ import annotations

from dataclasses import dataclass, field
from datetime import UTC, datetime
from typing import TYPE_CHECKING

from tools import cla

if TYPE_CHECKING:
    from collections.abc import Callable

DOCUMENT = cla.Document(url="https://example.test/CLA.md", sha256="0" * 64)
NOW = datetime(2026, 11, 24, 12, 0, tzinfo=UTC)
EXPECTED_COMMENT_COUNT_WITH_FORGERY = 2


def user(login: str, user_id: int, kind: str = "User") -> cla.JsonObject:
    return {"login": login, "id": user_id, "type": kind}


def commit(
    sha: str,
    email: str,
    account: cla.JsonObject | None,
    message: str = "change",
) -> cla.JsonObject:
    return {
        "sha": sha,
        "author": account,
        "commit": {"author": {"email": email}, "message": message},
    }


def pull(opener: cla.JsonObject, commit_count: int, number: int = 7) -> cla.JsonObject:
    return {"number": number, "user": opener, "commits": commit_count, "head": {"sha": "head-sha"}}


def assert_cla_error(action: Callable[[], object], fragment: str) -> None:
    caught: str | None = None
    try:
        action()
    except cla.ClaError as error:
        caught = str(error)
    assert caught is not None
    assert fragment in caught


def no_lookup(login: str) -> int | None:
    message = f"unexpected lookup of {login}"
    raise AssertionError(message)


@dataclass
class FakeGitHub:
    pull_request: cla.JsonObject
    pull_commits: list[cla.JsonObject]
    stored: list[cla.Signature] = field(default_factory=list)
    store_sha: str | None = None
    conflicts: int = 0
    users: dict[str, int] = field(default_factory=dict)
    statuses: list[tuple[str, str, str]] = field(default_factory=list)
    posted: list[cla.JsonObject] = field(default_factory=list)
    edits: list[tuple[int, str]] = field(default_factory=list)

    def pull(self, _number: int) -> cla.JsonObject:
        return self.pull_request

    def commits(self, _number: int) -> list[cla.JsonObject]:
        return self.pull_commits

    def user_id(self, login: str) -> int | None:
        return self.users.get(login)

    def signatures(self) -> tuple[list[cla.Signature], str | None]:
        return list(self.stored), self.store_sha

    def write_signatures(
        self, signatures: list[cla.Signature], sha: str | None, _message: str
    ) -> bool:
        assert sha == self.store_sha
        if self.conflicts:
            self.conflicts -= 1
            self.store_sha = f"moved-{self.conflicts}"
            return False
        self.stored = signatures
        self.store_sha = f"sha-{len(signatures)}"
        return True

    def set_status(self, sha: str, state: str, description: str, target_url: str) -> None:
        assert target_url == DOCUMENT.url
        self.statuses.append((sha, state, description))

    def comments(self, _number: int) -> list[cla.JsonObject]:
        return self.posted

    def create_comment(self, _number: int, body: str) -> None:
        self.posted.append(
            {"id": len(self.posted) + 1, "user": user(cla.COMMENT_AUTHOR, 1), "body": body}
        )

    def update_comment(self, comment_id: int, body: str) -> None:
        self.edits.append((comment_id, body))
        for comment in self.posted:
            if comment["id"] == comment_id:
                comment["body"] = body


def pr_event(number: int = 7) -> cla.JsonObject:
    return {"pull_request": {"number": number}}


def comment_event(body: str, author: cla.JsonObject, number: int = 7) -> cla.JsonObject:
    return {
        "issue": {"number": number, "pull_request": {"url": "https://example.test/pr"}},
        "comment": {"body": body, "user": author, "html_url": "https://example.test/c/1"},
    }


def signature_for(person: cla.JsonObject) -> cla.Signature:
    return cla.Signature(
        login=str(person["login"]),
        user_id=int(str(person["id"])),
        signed_at="2026-01-01T00:00:00+00:00",
        pull_request=1,
        comment_url="https://example.test/c/0",
        comment_body=cla.SIGN_PHRASE,
        document_sha256="0" * 64,
    )


ALICE = user("alice", 101)
BOB = user("bob", 202)


def test_signature_phrase_tolerates_whitespace_case_and_final_period() -> None:
    assert cla.is_signature(
        "  i have read the UNITAS contributor license\nagreement and I hereby sign it "
    )
    assert not cla.is_signature(f"{cla.SIGN_PHRASE} but not really")
    assert cla.is_recheck(" Recheck ")


def test_coauthor_trailers_are_extracted_case_insensitively() -> None:
    message = "Fix\n\nCo-authored-by: Bob <Bob@Example.org>\nco-authored-by: Eve <eve@x.io>\n"
    assert cla.coauthor_emails(message) == ["bob@example.org", "eve@x.io"]


def test_opener_and_commit_authors_are_required_once_and_bots_are_exempt() -> None:
    commits = [
        commit("a" * 40, "alice@example.org", ALICE),
        commit("b" * 40, "bob@example.org", BOB),
        commit("c" * 40, "alice@example.org", ALICE),
        commit("d" * 40, "bot@example.org", user("dependabot[bot]", 9, "Bot")),
        commit("e" * 40, "noreply@github.com", user("web-flow", 19864447)),
    ]
    requirement = cla.required_signers(pull(ALICE, 5), commits, no_lookup)
    assert set(requirement.contributors) == {101, 202}
    assert requirement.unresolved == {}


def test_commit_without_linked_account_is_unresolved() -> None:
    commits = [commit("ghostcommit1", "ghost@example.org", None)]
    requirement = cla.required_signers(pull(ALICE, 1), commits, no_lookup)
    assert list(requirement.unresolved.values()) == [
        cla.Unresolved("ghost@example.org", "author of commit ghostco")
    ]


def test_coauthors_resolve_through_noreply_known_emails_or_lookup() -> None:
    lines = [
        "Change",
        "",
        "Co-authored-by: Bob <bob@example.org>",
        "Co-authored-by: Carol <303+carol@users.noreply.github.com>",
        "Co-authored-by: Dan <dan@users.noreply.github.com>",
        "Co-authored-by: Claude <noreply@anthropic.com>",
        "Co-authored-by: Helper <99+helper[bot]@users.noreply.github.com>",
        "Co-authored-by: Erin <erin@example.org>",
    ]
    message = "\n".join(lines)
    commits = [
        commit("1" * 40, "bob@example.org", BOB),
        commit("2" * 40, "alice@example.org", ALICE, message),
    ]

    def lookup(login: str) -> int | None:
        return {"dan": 404}.get(login)

    requirement = cla.required_signers(pull(ALICE, 2), commits, lookup)
    assert set(requirement.contributors) == {101, 202, 303, 404}
    assert list(requirement.unresolved) == ["erin@example.org"]


def test_unknown_noreply_login_is_unresolved() -> None:
    message = "Change\n\nCo-authored-by: Gone <gone@users.noreply.github.com>"
    commits = [commit("3" * 40, "alice@example.org", ALICE, message)]
    requirement = cla.required_signers(pull(ALICE, 1), commits, lambda _login: None)
    assert list(requirement.unresolved) == ["gone@users.noreply.github.com"]


def test_unsigned_pull_request_fails_status_and_explains_how_to_sign() -> None:
    api = FakeGitHub(pull(ALICE, 1), [commit("4" * 40, "alice@example.org", ALICE)])
    verdict = cla.handle("pull_request_target", pr_event(), api, DOCUMENT, NOW)
    assert verdict is not None
    assert not verdict.accepted
    assert api.statuses == [("head-sha", "failure", "1 contributor needs to sign the CLA.")]
    assert len(api.posted) == 1
    body = str(api.posted[0]["body"])
    assert body.startswith(cla.COMMENT_MARKER)
    assert cla.SIGN_PHRASE in body
    assert "- [ ] @alice" in body


def test_signing_comment_records_signature_and_passes() -> None:
    api = FakeGitHub(pull(ALICE, 1), [commit("5" * 40, "alice@example.org", ALICE)])
    cla.handle("pull_request_target", pr_event(), api, DOCUMENT, NOW)
    verdict = cla.handle("issue_comment", comment_event(cla.SIGN_PHRASE, ALICE), api, DOCUMENT, NOW)
    assert verdict is not None
    assert verdict.accepted
    assert [(item.login, item.user_id, item.pull_request) for item in api.stored] == [
        ("alice", 101, 7)
    ]
    assert api.stored[0].signed_at == "2026-11-24T12:00:00+00:00"
    assert api.stored[0].document_sha256 == DOCUMENT.sha256
    assert api.statuses[-1] == ("head-sha", "success", "All contributors have signed the CLA.")
    assert len(api.posted) == 1
    assert api.edits
    assert "Thank you!" in api.edits[-1][1]


def test_signature_from_someone_outside_the_pull_request_is_ignored() -> None:
    api = FakeGitHub(pull(ALICE, 1), [commit("6" * 40, "alice@example.org", ALICE)])
    verdict = cla.handle("issue_comment", comment_event(cla.SIGN_PHRASE, BOB), api, DOCUMENT, NOW)
    assert verdict is not None
    assert not verdict.accepted
    assert api.stored == []


def test_signature_write_retries_after_a_concurrent_update() -> None:
    api = FakeGitHub(pull(ALICE, 1), [commit("7" * 40, "alice@example.org", ALICE)], conflicts=2)
    verdict = cla.handle("issue_comment", comment_event(cla.SIGN_PHRASE, ALICE), api, DOCUMENT, NOW)
    assert verdict is not None
    assert verdict.accepted
    assert len(api.stored) == 1


def test_signature_write_gives_up_when_the_store_never_settles() -> None:
    api = FakeGitHub(
        pull(ALICE, 1),
        [commit("8" * 40, "alice@example.org", ALICE)],
        conflicts=cla.WRITE_ATTEMPTS,
    )
    assert_cla_error(
        lambda: cla.handle(
            "issue_comment", comment_event(cla.SIGN_PHRASE, ALICE), api, DOCUMENT, NOW
        ),
        "kept changing",
    )


def test_partially_signed_pull_request_lists_remaining_signers() -> None:
    commits = [
        commit("9" * 40, "alice@example.org", ALICE),
        commit("a" * 40, "bob@example.org", BOB),
    ]
    api = FakeGitHub(pull(ALICE, 2), commits, stored=[signature_for(ALICE)], store_sha="s")
    verdict = cla.handle("pull_request_target", pr_event(), api, DOCUMENT, NOW)
    assert verdict is not None
    assert [person.login for person in verdict.missing] == ["bob"]
    body = str(api.posted[0]["body"])
    assert "- [x] @alice" in body
    assert "- [ ] @bob" in body


def test_signed_pull_request_without_prior_comment_stays_quiet() -> None:
    api = FakeGitHub(
        pull(ALICE, 1),
        [commit("b" * 40, "alice@example.org", ALICE)],
        stored=[signature_for(ALICE)],
        store_sha="s",
    )
    verdict = cla.handle("pull_request_target", pr_event(), api, DOCUMENT, NOW)
    assert verdict is not None
    assert verdict.accepted
    assert api.posted == []
    assert api.statuses == [("head-sha", "success", "All contributors have signed the CLA.")]


def test_status_comment_from_another_author_is_not_reused() -> None:
    api = FakeGitHub(pull(ALICE, 1), [commit("c" * 40, "alice@example.org", ALICE)])
    api.posted.append({"id": 50, "user": BOB, "body": cla.COMMENT_MARKER + "\nforged"})
    cla.handle("pull_request_target", pr_event(), api, DOCUMENT, NOW)
    assert api.edits == []
    assert len(api.posted) == EXPECTED_COMMENT_COUNT_WITH_FORGERY


def test_unresolved_identity_blocks_even_when_everyone_signed() -> None:
    commits = [
        commit("d" * 40, "alice@example.org", ALICE),
        commit("e" * 40, "ghost@example.org", None),
    ]
    api = FakeGitHub(pull(ALICE, 2), commits, stored=[signature_for(ALICE)], store_sha="s")
    verdict = cla.handle("pull_request_target", pr_event(), api, DOCUMENT, NOW)
    assert verdict is not None
    assert not verdict.accepted
    assert api.statuses[-1][1] == "failure"
    assert "`ghost@example.org` (author of commit eeeeeee)" in str(api.posted[0]["body"])


def test_irrelevant_comments_and_issue_comments_are_ignored() -> None:
    api = FakeGitHub(pull(ALICE, 1), [])
    assert (
        cla.handle("issue_comment", comment_event("looks good", ALICE), api, DOCUMENT, NOW) is None
    )
    issue_only: cla.JsonObject = {
        "issue": {"number": 3},
        "comment": {"body": cla.SIGN_PHRASE, "user": ALICE, "html_url": "x"},
    }
    assert cla.handle("issue_comment", issue_only, api, DOCUMENT, NOW) is None
    assert api.statuses == []


def test_pull_request_with_unlisted_commits_is_rejected() -> None:
    api = FakeGitHub(pull(ALICE, 300), [commit("f" * 40, "alice@example.org", ALICE)])
    assert_cla_error(
        lambda: cla.handle("pull_request_target", pr_event(), api, DOCUMENT, NOW), "split it"
    )


def test_unsupported_event_is_an_error() -> None:
    api = FakeGitHub(pull(ALICE, 1), [])
    assert_cla_error(lambda: cla.handle("push", {}, api, DOCUMENT, NOW), "unsupported event")


def test_signature_store_round_trips() -> None:
    signatures = [signature_for(ALICE), signature_for(BOB)]
    assert cla.parse_signatures(cla.render_signatures(signatures)) == signatures


def test_status_description_fits_the_api_limit() -> None:
    many = tuple(cla.Contributor(f"user{index}", index) for index in range(10**3))
    verdict = cla.Verdict(signed=(), missing=many, unresolved=())
    assert len(cla.status_description(verdict)) <= cla.STATUS_DESCRIPTION_LIMIT


def test_published_documents_quote_the_exact_signing_phrase() -> None:
    for name in (cla.DOCUMENT_NAME, "CONTRIBUTING.md"):
        text = (cla.REPO_ROOT / name).read_text(encoding="utf-8")
        assert cla.SIGN_PHRASE in text, name
