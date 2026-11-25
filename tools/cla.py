from __future__ import annotations

import base64
import hashlib
import http.client
import json
import os
import re
import sys
import urllib.parse
from dataclasses import asdict, dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from collections.abc import Callable, Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
DOCUMENT_NAME = "CLA.md"
DOCUMENT_PATH = REPO_ROOT / DOCUMENT_NAME
SIGNATURES_BRANCH = "cla-signatures"
SIGNATURES_PATH = "signatures.json"
STATUS_CONTEXT = "cla"
SIGN_PHRASE = "I have read the unitas Contributor License Agreement and I hereby sign it."
RECHECK_PHRASE = "recheck"
COMMENT_MARKER = "<!-- unitas-cla-status -->"
COMMENT_AUTHOR = "github-actions[bot]"
EXEMPT_LOGINS = frozenset({"web-flow", "copilot", "claude"})
EXEMPT_EMAILS = frozenset({"noreply@anthropic.com", "cursoragent@cursor.com", "noreply@github.com"})
NOREPLY_PATTERN = re.compile(
    r"^(?:(?P<id>\d+)\+)?(?P<login>[a-z0-9-]+(?:\[bot\])?)@users\.noreply\.github\.com$"
)
COAUTHOR_PATTERN = re.compile(
    r"^\s*co-authored-by:\s*(?P<name>.*?)\s*<(?P<email>[^<>\s]+)>\s*$",
    re.IGNORECASE | re.MULTILINE,
)
API_HOST = "api.github.com"
API_TIMEOUT_SECONDS = 30
COMMITS_PER_PAGE = 100
MAX_LISTED_COMMITS = 250
WRITE_ATTEMPTS = 3
STATUS_DESCRIPTION_LIMIT = 140
HTTP_REDIRECT_THRESHOLD = 300
HTTP_NOT_FOUND = 404
HTTP_CONFLICT = 409
COMMENTS_PER_PAGE = 100

JsonObject = dict[str, object]


class ClaError(Exception):
    pass


def as_object(value: object, where: str) -> JsonObject:
    if not isinstance(value, dict):
        message = f"expected a JSON object at {where}"
        raise ClaError(message)
    return {str(key): item for key, item in value.items()}


def as_optional_object(value: object, where: str) -> JsonObject | None:
    if value is None:
        return None
    return as_object(value, where)


def as_list(value: object, where: str) -> list[object]:
    if not isinstance(value, list):
        message = f"expected a JSON array at {where}"
        raise ClaError(message)
    return list(value)


def as_text(value: object, where: str) -> str:
    if not isinstance(value, str):
        message = f"expected a string at {where}"
        raise ClaError(message)
    return value


def as_int(value: object, where: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        message = f"expected an integer at {where}"
        raise ClaError(message)
    return value


@dataclass(frozen=True)
class Contributor:
    login: str
    user_id: int


@dataclass(frozen=True)
class Unresolved:
    email: str
    source: str


@dataclass
class Requirement:
    contributors: dict[int, Contributor] = field(default_factory=dict)
    unresolved: dict[str, Unresolved] = field(default_factory=dict)

    def require(self, contributor: Contributor) -> None:
        self.contributors.setdefault(contributor.user_id, contributor)

    def flag(self, email: str, source: str) -> None:
        self.unresolved.setdefault(email, Unresolved(email, source))


@dataclass(frozen=True)
class Signature:
    login: str
    user_id: int
    signed_at: str
    pull_request: int
    comment_url: str
    comment_body: str
    document_sha256: str


@dataclass(frozen=True)
class Verdict:
    signed: tuple[Contributor, ...]
    missing: tuple[Contributor, ...]
    unresolved: tuple[Unresolved, ...]

    @property
    def accepted(self) -> bool:
        return not self.missing and not self.unresolved


@dataclass(frozen=True)
class Document:
    url: str
    sha256: str


class GitHub(Protocol):
    def pull(self, number: int) -> JsonObject: ...

    def commits(self, number: int) -> list[JsonObject]: ...

    def user_id(self, login: str) -> int | None: ...

    def signatures(self) -> tuple[list[Signature], str | None]: ...

    def write_signatures(
        self, signatures: list[Signature], sha: str | None, message: str
    ) -> bool: ...

    def set_status(self, sha: str, state: str, description: str, target_url: str) -> None: ...

    def comments(self, number: int) -> list[JsonObject]: ...

    def create_comment(self, number: int, body: str) -> None: ...

    def update_comment(self, comment_id: int, body: str) -> None: ...


def normalized(text: str) -> str:
    return " ".join(text.split()).casefold().rstrip(".")


def is_signature(body: str) -> bool:
    return normalized(body) == normalized(SIGN_PHRASE)


def is_recheck(body: str) -> bool:
    return normalized(body) == RECHECK_PHRASE


def coauthor_emails(message: str) -> list[str]:
    return [match.group("email").lower() for match in COAUTHOR_PATTERN.finditer(message)]


def is_exempt_login(login: str) -> bool:
    lowered = login.lower()
    return lowered.endswith("[bot]") or lowered in EXEMPT_LOGINS


def contributor_from_user(user: JsonObject, where: str) -> Contributor | None:
    login = as_text(user.get("login"), f"{where}.login")
    if user.get("type") == "Bot" or is_exempt_login(login):
        return None
    return Contributor(login, as_int(user.get("id"), f"{where}.id"))


def resolve_coauthor(
    email: str,
    known: dict[str, Contributor | None],
    lookup: Callable[[str], int | None],
) -> tuple[bool, Contributor | None]:
    if email in EXEMPT_EMAILS:
        return True, None
    if email in known:
        return True, known[email]
    noreply = NOREPLY_PATTERN.match(email)
    if noreply is None:
        return False, None
    login = noreply.group("login")
    if is_exempt_login(login):
        return True, None
    digits = noreply.group("id")
    user_id = int(digits) if digits else lookup(login)
    if user_id is None:
        return False, None
    return True, Contributor(login, user_id)


def required_signers(
    pull: JsonObject,
    commits: Sequence[JsonObject],
    lookup: Callable[[str], int | None],
) -> Requirement:
    requirement = Requirement()
    opener = contributor_from_user(as_object(pull.get("user"), "pull.user"), "pull.user")
    if opener is not None:
        requirement.require(opener)
    known: dict[str, Contributor | None] = {}
    for index, commit in enumerate(commits):
        where = f"commits[{index}]"
        short_sha = as_text(commit.get("sha"), f"{where}.sha")[:7]
        details = as_object(commit.get("commit"), f"{where}.commit")
        git_author = as_object(details.get("author"), f"{where}.commit.author")
        email = as_text(git_author.get("email"), f"{where}.commit.author.email").lower()
        account = as_optional_object(commit.get("author"), f"{where}.author")
        if account is None:
            if email not in EXEMPT_EMAILS:
                requirement.flag(email, f"author of commit {short_sha}")
            continue
        author = contributor_from_user(account, f"{where}.author")
        known[email] = author
        if author is not None:
            requirement.require(author)
    for index, commit in enumerate(commits):
        where = f"commits[{index}]"
        short_sha = as_text(commit.get("sha"), f"{where}.sha")[:7]
        details = as_object(commit.get("commit"), f"{where}.commit")
        message = as_text(details.get("message"), f"{where}.commit.message")
        for email in coauthor_emails(message):
            resolved, coauthor = resolve_coauthor(email, known, lookup)
            if not resolved:
                requirement.flag(email, f"co-author of commit {short_sha}")
            elif coauthor is not None:
                requirement.require(coauthor)
    return requirement


def evaluate(requirement: Requirement, signed_ids: set[int]) -> Verdict:
    ordered = sorted(requirement.contributors.values(), key=lambda person: person.login.lower())
    return Verdict(
        signed=tuple(person for person in ordered if person.user_id in signed_ids),
        missing=tuple(person for person in ordered if person.user_id not in signed_ids),
        unresolved=tuple(sorted(requirement.unresolved.values(), key=lambda item: item.email)),
    )


def parse_signatures(raw: bytes) -> list[Signature]:
    data = as_object(json.loads(raw.decode("utf-8")), "signatures file")
    entries = as_list(data.get("signatures"), "signatures file.signatures")
    signatures: list[Signature] = []
    for index, entry in enumerate(entries):
        where = f"signatures[{index}]"
        record = as_object(entry, where)
        signatures.append(
            Signature(
                login=as_text(record.get("login"), f"{where}.login"),
                user_id=as_int(record.get("user_id"), f"{where}.user_id"),
                signed_at=as_text(record.get("signed_at"), f"{where}.signed_at"),
                pull_request=as_int(record.get("pull_request"), f"{where}.pull_request"),
                comment_url=as_text(record.get("comment_url"), f"{where}.comment_url"),
                comment_body=as_text(record.get("comment_body"), f"{where}.comment_body"),
                document_sha256=as_text(record.get("document_sha256"), f"{where}.document_sha256"),
            )
        )
    return signatures


def render_signatures(signatures: list[Signature]) -> bytes:
    payload = {
        "document": DOCUMENT_NAME,
        "signatures": [asdict(signature) for signature in signatures],
    }
    return (json.dumps(payload, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def record_signature(api: GitHub, signature: Signature) -> bool:
    for _ in range(WRITE_ATTEMPTS):
        signatures, sha = api.signatures()
        if any(existing.user_id == signature.user_id for existing in signatures):
            return False
        message = f"Record CLA signature of @{signature.login} from #{signature.pull_request}"
        if api.write_signatures([*signatures, signature], sha, message):
            return True
    failure = f"could not record the signature of {signature.login}: the store kept changing"
    raise ClaError(failure)


def status_description(verdict: Verdict) -> str:
    if verdict.accepted:
        text = "All contributors have signed the CLA."
    else:
        pending = len(verdict.missing) + len(verdict.unresolved)
        noun = "contributor needs" if pending == 1 else "contributors need"
        text = f"{pending} {noun} to sign the CLA."
    return text[:STATUS_DESCRIPTION_LIMIT]


def render_comment(verdict: Verdict, document: Document) -> str:
    lines = [COMMENT_MARKER]
    if verdict.accepted:
        lines.append(
            f"All contributors to this pull request have signed the "
            f"[unitas Contributor License Agreement]({document.url}). Thank you!"
        )
        return "\n".join(lines) + "\n"
    lines += [
        "Thank you for your contribution!",
        "",
        "unitas is dual licensed under GPL-3.0 and a commercial license, so every "
        f"contributor must sign the [unitas Contributor License Agreement]({document.url}) "
        "before this pull request can be merged. You only sign once; the signature "
        "covers your future contributions too.",
    ]
    if verdict.missing:
        lines += [
            "",
            "To sign, post a comment on this pull request containing exactly:",
            "",
            "```",
            SIGN_PHRASE,
            "```",
            "",
        ]
        lines += [f"- [x] @{person.login}" for person in verdict.signed]
        lines += [f"- [ ] @{person.login}" for person in verdict.missing]
    if verdict.unresolved:
        lines += [
            "",
            "These commit identities are not linked to a GitHub account, so their "
            "signature cannot be checked:",
            "",
        ]
        lines += [f"- `{item.email}` ({item.source})" for item in verdict.unresolved]
        lines += [
            "",
            "Add the email address to the GitHub account of the person who wrote the "
            "change (Settings, Emails) or rewrite the commit author, then sign if "
            f"needed and comment `{RECHECK_PHRASE}`.",
        ]
    return "\n".join(lines) + "\n"


def upsert_comment(api: GitHub, number: int, verdict: Verdict, document: Document) -> None:
    body = render_comment(verdict, document)
    for index, comment in enumerate(api.comments(number)):
        where = f"comments[{index}]"
        author = as_object(comment.get("user"), f"{where}.user")
        text = as_text(comment.get("body"), f"{where}.body")
        if author.get("login") != COMMENT_AUTHOR or not text.startswith(COMMENT_MARKER):
            continue
        if text != body:
            api.update_comment(as_int(comment.get("id"), f"{where}.id"), body)
        return
    if not verdict.accepted:
        api.create_comment(number, body)


def signature_from_comment(
    comment: JsonObject,
    requirement: Requirement,
    number: int,
    document: Document,
    now: datetime,
) -> Signature | None:
    body = as_text(comment.get("body"), "comment.body")
    if not is_signature(body):
        return None
    user = as_object(comment.get("user"), "comment.user")
    signer = requirement.contributors.get(as_int(user.get("id"), "comment.user.id"))
    if signer is None:
        return None
    return Signature(
        login=signer.login,
        user_id=signer.user_id,
        signed_at=now.astimezone(UTC).isoformat(timespec="seconds"),
        pull_request=number,
        comment_url=as_text(comment.get("html_url"), "comment.html_url"),
        comment_body=body,
        document_sha256=document.sha256,
    )


def target_pull_request(event_name: str, event: JsonObject) -> tuple[int, JsonObject | None] | None:
    if event_name == "pull_request_target":
        pull = as_object(event.get("pull_request"), "event.pull_request")
        return as_int(pull.get("number"), "event.pull_request.number"), None
    if event_name != "issue_comment":
        message = f"unsupported event {event_name}"
        raise ClaError(message)
    issue = as_object(event.get("issue"), "event.issue")
    if issue.get("pull_request") is None:
        return None
    comment = as_object(event.get("comment"), "event.comment")
    body = as_text(comment.get("body"), "event.comment.body")
    if not is_signature(body) and not is_recheck(body):
        return None
    return as_int(issue.get("number"), "event.issue.number"), comment


def handle(
    event_name: str,
    event: JsonObject,
    api: GitHub,
    document: Document,
    now: datetime,
) -> Verdict | None:
    target = target_pull_request(event_name, event)
    if target is None:
        return None
    number, comment = target
    pull = api.pull(number)
    commits = api.commits(number)
    if as_int(pull.get("commits"), "pull.commits") > len(commits):
        message = (
            f"pull request #{number} has more commits than the API lists "
            f"({MAX_LISTED_COMMITS}); split it into smaller pull requests"
        )
        raise ClaError(message)
    requirement = required_signers(pull, commits, api.user_id)
    if comment is not None:
        signature = signature_from_comment(comment, requirement, number, document, now)
        if signature is not None:
            record_signature(api, signature)
    signed_ids = {signature.user_id for signature in api.signatures()[0]}
    verdict = evaluate(requirement, signed_ids)
    head = as_object(pull.get("head"), "pull.head")
    api.set_status(
        as_text(head.get("sha"), "pull.head.sha"),
        "success" if verdict.accepted else "failure",
        status_description(verdict),
        document.url,
    )
    upsert_comment(api, number, verdict, document)
    return verdict


class RestGitHub:
    def __init__(self, token: str, repository: str) -> None:
        self.token = token
        self.repository = repository

    def call(self, method: str, path: str, payload: JsonObject | None = None) -> tuple[int, object]:
        data = None if payload is None else json.dumps(payload).encode("utf-8")
        headers = {
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {self.token}",
            "Content-Type": "application/json",
            "User-Agent": "unitas-cla-check",
            "X-GitHub-Api-Version": "2022-11-28",
        }
        connection = http.client.HTTPSConnection(API_HOST, timeout=API_TIMEOUT_SECONDS)
        try:
            connection.request(method, path, body=data, headers=headers)
            response = connection.getresponse()
            return response.status, decode_body(response.read())
        finally:
            connection.close()

    def expect(self, method: str, path: str, payload: JsonObject | None = None) -> object:
        status, body = self.call(method, path, payload)
        if status >= HTTP_REDIRECT_THRESHOLD:
            message = f"{method} {path} failed with HTTP {status}: {body}"
            raise ClaError(message)
        return body

    def repo_path(self, suffix: str) -> str:
        return f"/repos/{self.repository}{suffix}"

    def pull(self, number: int) -> JsonObject:
        return as_object(self.expect("GET", self.repo_path(f"/pulls/{number}")), "pull")

    def commits(self, number: int) -> list[JsonObject]:
        listed: list[JsonObject] = []
        page = 1
        while len(listed) < MAX_LISTED_COMMITS:
            query = f"?per_page={COMMITS_PER_PAGE}&page={page}"
            body = self.expect("GET", self.repo_path(f"/pulls/{number}/commits{query}"))
            batch = [as_object(item, "commit") for item in as_list(body, "commits")]
            listed.extend(batch)
            if len(batch) < COMMITS_PER_PAGE:
                break
            page += 1
        return listed

    def user_id(self, login: str) -> int | None:
        status, body = self.call("GET", f"/users/{urllib.parse.quote(login)}")
        if status == HTTP_NOT_FOUND:
            return None
        if status >= HTTP_REDIRECT_THRESHOLD:
            message = f"looking up user {login} failed with HTTP {status}: {body}"
            raise ClaError(message)
        return as_int(as_object(body, "user").get("id"), "user.id")

    def signatures(self) -> tuple[list[Signature], str | None]:
        query = f"?ref={SIGNATURES_BRANCH}"
        status, body = self.call("GET", self.repo_path(f"/contents/{SIGNATURES_PATH}{query}"))
        if status == HTTP_NOT_FOUND:
            return [], None
        if status >= HTTP_REDIRECT_THRESHOLD:
            message = f"reading {SIGNATURES_PATH} failed with HTTP {status}: {body}"
            raise ClaError(message)
        content = as_object(body, "contents")
        raw = base64.b64decode(as_text(content.get("content"), "contents.content"))
        return parse_signatures(raw), as_text(content.get("sha"), "contents.sha")

    def write_signatures(self, signatures: list[Signature], sha: str | None, message: str) -> bool:
        payload: JsonObject = {
            "message": message,
            "branch": SIGNATURES_BRANCH,
            "content": base64.b64encode(render_signatures(signatures)).decode("ascii"),
        }
        if sha is not None:
            payload["sha"] = sha
        status, body = self.call("PUT", self.repo_path(f"/contents/{SIGNATURES_PATH}"), payload)
        if status == HTTP_CONFLICT:
            return False
        if status >= HTTP_REDIRECT_THRESHOLD:
            failure = (
                f"writing {SIGNATURES_PATH} to branch {SIGNATURES_BRANCH} failed "
                f"with HTTP {status}: {body}"
            )
            raise ClaError(failure)
        return True

    def set_status(self, sha: str, state: str, description: str, target_url: str) -> None:
        payload: JsonObject = {
            "state": state,
            "context": STATUS_CONTEXT,
            "description": description,
            "target_url": target_url,
        }
        self.expect("POST", self.repo_path(f"/statuses/{sha}"), payload)

    def comments(self, number: int) -> list[JsonObject]:
        listed: list[JsonObject] = []
        page = 1
        while True:
            query = f"?per_page={COMMENTS_PER_PAGE}&page={page}"
            body = self.expect("GET", self.repo_path(f"/issues/{number}/comments{query}"))
            batch = [as_object(item, "comment") for item in as_list(body, "comments")]
            listed.extend(batch)
            if len(batch) < COMMENTS_PER_PAGE:
                return listed
            page += 1

    def create_comment(self, number: int, body: str) -> None:
        self.expect("POST", self.repo_path(f"/issues/{number}/comments"), {"body": body})

    def update_comment(self, comment_id: int, body: str) -> None:
        self.expect("PATCH", self.repo_path(f"/issues/comments/{comment_id}"), {"body": body})


def decode_body(raw: bytes) -> object:
    if not raw:
        return None
    decoded: object = json.loads(raw.decode("utf-8"))
    return decoded


def required_setting(name: str) -> str:
    value = os.getenv(name)
    if not value:
        message = f"environment variable {name} is not set"
        raise ClaError(message)
    return value


def main() -> int:
    try:
        token = required_setting("GITHUB_TOKEN")
        repository = required_setting("GITHUB_REPOSITORY")
        event_name = required_setting("GITHUB_EVENT_NAME")
        event_path = Path(required_setting("GITHUB_EVENT_PATH"))
        server = os.getenv("GITHUB_SERVER_URL") or "https://github.com"
        revision = required_setting("GITHUB_SHA")
        event = as_object(json.loads(event_path.read_text(encoding="utf-8")), "event")
        document = Document(
            url=f"{server}/{repository}/blob/{revision}/{DOCUMENT_NAME}",
            sha256=hashlib.sha256(DOCUMENT_PATH.read_bytes()).hexdigest(),
        )
        verdict = handle(
            event_name, event, RestGitHub(token, repository), document, datetime.now(UTC)
        )
    except ClaError as error:
        sys.stderr.write(f"cla: {error}\n")
        return 1
    if verdict is None:
        sys.stdout.write("cla: event does not concern a pull request signature check\n")
    else:
        sys.stdout.write(f"cla: {status_description(verdict)}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
