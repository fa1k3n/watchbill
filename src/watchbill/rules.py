"""The rule table and the model table of the revision: read, and checked.

This module only reads and checks. Nothing here acts on a rule. It is the only module
that reads the two tables, and it reads them through `watchbill.yamlfile`.
"""

import os
import re
from dataclasses import dataclass
from typing import Dict, List

from watchbill import yamlfile
from watchbill.yamlfile import YamlError

__all__ = ["ROLES", "EFFORTS", "RULES_FILE", "MODELS_FILE", "Role", "Model", "Rules", "YamlError", "load"]

# The roles of the first revision. The officer has rows and no prompt yet.
ROLES = ("tester", "build", "review", "fix", "officer")
ROLE_FIELDS = ("step", "produces", "reads", "may", "may_not_also_hold", "prompt")
EFFORTS = ("low", "medium", "high", "xhigh", "max")

REVISION_FOLDER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "revision")
RULES_FILE = os.path.join(REVISION_FOLDER, "rules.yml")
MODELS_FILE = os.path.join(REVISION_FOLDER, "models.yml")

# A name and a version: it ends in a number, and it is not "latest".
EXACT_MODEL = re.compile(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*-[0-9]+$")
MERGE = re.compile(r"\bmerge\b")


@dataclass(frozen=True)
class Role:
    name: str
    step: str
    produces: str
    reads: List[str]
    may: List[str]
    may_not_also_hold: List[str]
    prompt: str  # a file below the revision folder, or "none"


@dataclass(frozen=True)
class Model:
    model: str
    effort: str


@dataclass(frozen=True)
class Rules:
    review_rounds: int
    always_to_a_person: List[str]
    classed_code_verifier: str
    officer_queues: List[str]
    roles: Dict[str, Role]
    models: Dict[str, Model]


def _role(doc, name):
    place = ("roles", name)
    doc.only(place, ROLE_FIELDS)
    doc.need(place, ROLE_FIELDS)
    prompt = doc.text(place + ("prompt",))
    if prompt != "none" and (not prompt.endswith(".md") or prompt.startswith("/") or ".." in prompt.split("/")):
        doc.fail(place + ("prompt",), "`none` or a file below the revision folder, such as roles/%s.md, is expected" % name)
    row = Role(
        name=name,
        step=doc.text(place + ("step",)),
        produces=doc.text(place + ("produces",)),
        reads=doc.texts(place + ("reads",)),
        may=doc.texts(place + ("may",)),
        may_not_also_hold=doc.texts(place + ("may_not_also_hold",)),
        prompt=prompt,
    )
    for index, entry in enumerate(row.may):
        if MERGE.search(entry.lower()):
            doc.fail(place + ("may", index), "%s may not merge: no agent does, and a person is the one who merges" % name)
    for index, other in enumerate(row.may_not_also_hold):
        where = place + ("may_not_also_hold", index)
        if other == name:
            doc.fail(where, "%s cannot exclude itself" % name)
        if other not in ROLES:
            doc.fail(where, "%s may not also hold `%s`, which is not one of the roles: %s" % (name, other, ", ".join(ROLES)))
    return row


def _read_rules(path):
    doc = yamlfile.load(path)
    top = ("review_rounds", "always_to_a_person", "classed_code", "officer_queues", "roles")
    doc.only((), top)
    doc.need((), top)
    rounds = doc.whole(("review_rounds",), minimum=1)
    person = doc.texts(("always_to_a_person",))
    if len(set(person)) != len(person):
        doc.fail(("always_to_a_person",), "a name is written twice")
    doc.only(("classed_code",), ("verifier",))
    doc.need(("classed_code",), ("verifier",))
    verifier = doc.text(("classed_code", "verifier"))
    if verifier != "person":
        doc.fail(("classed_code", "verifier"), "a change that touches classed code needs a person as its verifier, so `person` is expected")
    queues = doc.texts(("officer_queues",), empty=True)
    doc.only(("roles",), ROLES)
    doc.need(("roles",), ROLES)
    roles = {name: _role(doc, name) for name in ROLES}
    # "May not also hold" holds both ways.
    for name, role in roles.items():
        for other in role.may_not_also_hold:
            if name not in roles[other].may_not_also_hold:
                doc.fail(
                    ("roles", name, "may_not_also_hold"),
                    "%s may not also hold %s, but %s does not say the same of %s" % (name, other, other, name),
                )
    # The officer holds no other role on a task it has handed out.
    officer = roles["officer"]
    for other in ROLES:
        if other != "officer" and other not in officer.may_not_also_hold:
            doc.fail(("roles", "officer", "may_not_also_hold"), "the officer holds no other role, so `%s` is expected here" % other)
    return doc, Rules(rounds, person, verifier, queues, roles, {})


def _exact(model):
    return bool(EXACT_MODEL.match(model)) and "latest" not in model.split("-")


def _read_models(path, roles):
    doc = yamlfile.load(path)
    doc.only((), ("models",))
    doc.need((), ("models",))
    given = doc.table(("models",))
    with_prompt = [name for name in ROLES if roles[name].prompt != "none"]
    for name in given:
        if name not in ROLES:
            doc.fail(("models", name), "`%s` is not one of the roles: %s" % (name, ", ".join(ROLES)))
        if name not in with_prompt:
            doc.fail(("models", name), "%s has no prompt, so it has no row here" % name)
    for name in with_prompt:
        if name not in given:
            doc.fail(("models",), "%s has a prompt but no row here" % name)
    models = {}
    for name in with_prompt:
        place = ("models", name)
        doc.only(place, ("model", "effort"))
        doc.need(place, ("model", "effort"))
        model = doc.text(place + ("model",))
        if not _exact(model):
            doc.fail(
                place + ("model",),
                "`%s` is not an exact model ID: it needs a name and a version, such as claude-sonnet-5-5, and never `latest`" % model,
            )
        effort = doc.text(place + ("effort",))
        if effort not in EFFORTS:
            doc.fail(place + ("effort",), "`%s` is not an effort; it is one of: %s" % (effort, ", ".join(EFFORTS)))
        models[name] = Model(model, effort)
    return models


def load(rules_file=RULES_FILE, models_file=MODELS_FILE):
    """Read and check both tables. A table that is wrong raises `YamlError`, naming the file and the place."""
    _, table = _read_rules(rules_file)
    models = _read_models(models_file, table.roles)
    return Rules(
        table.review_rounds, table.always_to_a_person, table.classed_code_verifier,
        table.officer_queues, table.roles, models,
    )
