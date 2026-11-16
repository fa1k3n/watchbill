"""The one loader of YAML in `wb`.

Nothing else in `wb` imports the YAML library. A file is read with the safe loader,
and then it is held to what Watchbill asks of it:

- a key written twice is refused, at any depth (the library alone keeps the last one);
- a tag that would build more than a plain value is refused;
- a value is held to the type its field asks for, and where a text is expected and YAML
  read something else (`no`, `1.10`), the message says to put the value in quotes.

A fault is a `YamlError`. Its message names the file and the place, and is never a stack trace.
"""

import datetime

import yaml

__all__ = ["YamlError", "Document", "load"]

# What the safe loader builds beyond plain values: bytes, sets, ordered pairs.
_REFUSED_TAGS = {
    "tag:yaml.org,2002:binary",
    "tag:yaml.org,2002:set",
    "tag:yaml.org,2002:omap",
    "tag:yaml.org,2002:pairs",
}
_MERGE = "tag:yaml.org,2002:merge"


class YamlError(Exception):
    """A file that is refused. `str()` of it is the whole message for the user."""


class _Loader(yaml.SafeLoader):
    """The safe loader, with plain values only and a key written twice refused."""

    def construct_mapping(self, node, deep=False):
        if isinstance(node, yaml.MappingNode):
            seen = set()
            for key_node, _ in node.value:
                if key_node.tag == _MERGE:
                    continue
                key = self.construct_object(key_node, deep=True)
                try:
                    if key in seen:
                        raise yaml.constructor.ConstructorError(
                            None, None, "the key `%s` is written twice" % (key,), key_node.start_mark
                        )
                    seen.add(key)
                except TypeError:
                    break  # an unhashable key: the library says so itself
        return super().construct_mapping(node, deep)


_Loader.yaml_constructors = {
    tag: constructor
    for tag, constructor in yaml.SafeLoader.yaml_constructors.items()
    if tag not in _REFUSED_TAGS
}


def _kind(value):
    """What YAML read a scalar as, in words."""
    if value is None:
        return "nothing (null)"
    if isinstance(value, bool):
        return "a boolean"
    if isinstance(value, int):
        return "a whole number"
    if isinstance(value, float):
        return "a number"
    if isinstance(value, (datetime.date, datetime.datetime)):
        return "a date"
    if isinstance(value, str):
        return "a text"
    if isinstance(value, list):
        return "a list"
    if isinstance(value, dict):
        return "a mapping"
    return "something that is not a plain value"


def _walk(node, place, nodes, keys, active):
    """Remember the node of every place, so that a fault can name its line."""
    nodes[place] = node
    if id(node) in active:
        return
    active.add(id(node))
    if isinstance(node, yaml.MappingNode):
        for key_node, value_node in node.value:
            if key_node.tag == _MERGE:
                continue
            sub = place + (str(key_node.value),)
            keys[sub] = key_node
            _walk(value_node, sub, nodes, keys, active)
    elif isinstance(node, yaml.SequenceNode):
        for index, item in enumerate(node.value):
            _walk(item, place + (index,), nodes, keys, active)
    active.discard(id(node))


def _problem(path, exc):
    """The message for a fault of the library, with the line and column."""
    mark = getattr(exc, "problem_mark", None)
    problem = getattr(exc, "problem", None) or str(exc)
    problem = " ".join(str(problem).split())
    if "could not determine a constructor" in problem or "could not construct" in problem:
        problem = "this tag builds more than a plain value, and only plain values are read"
    if mark is None:
        return "%s: %s" % (path, problem)
    return "%s, line %d, column %d: %s" % (path, mark.line + 1, mark.column + 1, problem)


def load(path):
    """Read a YAML file into a `Document`, or raise `YamlError` naming the file."""
    try:
        with open(path, "rb") as file:
            raw = file.read()
    except OSError as exc:
        raise YamlError("%s: cannot be read (%s)" % (path, exc.strerror or exc.__class__.__name__))
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        raise YamlError("%s: is not text in UTF-8" % path)
    try:
        data = yaml.load(text, Loader=_Loader)
        root = yaml.compose(text, Loader=_Loader)
    except yaml.YAMLError as exc:
        raise YamlError(_problem(path, exc))
    nodes, keys = {}, {}
    if root is not None:
        _walk(root, (), nodes, keys, set())
    return Document(path, data, nodes, keys)


class Document:
    """What a file holds, with the means to hold it to its fields and to name a place in it.

    A place is a tuple of keys and list positions: `("roles", "build", "step")`.
    """

    def __init__(self, path, data, nodes, keys):
        self.path = path
        self.data = data
        self._nodes = nodes
        self._keys = keys

    # Where a thing is.

    @staticmethod
    def dotted(place):
        return ".".join(str(part) for part in place) if place else "the top of the file"

    def fail(self, place, message, key=False):
        """Refuse the file, naming the place and its line. With `key`, the line of the key."""
        node = (self._keys if key else self._nodes).get(tuple(place)) or self._nodes.get(tuple(place))
        line = ", line %d" % (node.start_mark.line + 1) if node is not None else ""
        raise YamlError("%s%s (%s): %s" % (self.path, line, self.dotted(tuple(place)), message))

    def value(self, place):
        """The value at a place, or raise `KeyError`."""
        found = self.data
        for part in place:
            if isinstance(found, dict):
                matches = [key for key in found if str(key) == str(part)]
                if not matches:
                    raise KeyError(part)
                found = found[matches[0]]
            elif isinstance(found, list) and isinstance(part, int) and 0 <= part < len(found):
                found = found[part]
            else:
                raise KeyError(part)
        return found

    def has(self, place):
        try:
            self.value(place)
        except KeyError:
            return False
        return True

    def _raw(self, place):
        node = self._nodes.get(tuple(place))
        return node.value if isinstance(node, yaml.ScalarNode) else None

    # Holding a value to its type.

    def text(self, place, blank=False):
        """A text. YAML reading `no`, `1.10` or nothing is refused with a word on quotes."""
        found = self.value(place)
        if not isinstance(found, str):
            if isinstance(found, (list, dict)):
                self.fail(place, "a text is expected, but a %s is here" % _kind(found).split(" ", 1)[1])
            raw = self._raw(place)
            written = " (written as `%s`)" % raw if raw else ""
            self.fail(
                place,
                "a text is expected, but YAML read it as %s%s. Put the value in quotes." % (_kind(found), written),
            )
        if not blank and not found.strip():
            self.fail(place, "a text is expected, and there is nothing in it")
        return found

    def whole(self, place, minimum=None):
        """A whole number, and at least `minimum` when it is given."""
        found = self.value(place)
        need = "a whole number" + (" of at least %d" % minimum if minimum is not None else "") + " is expected"
        if isinstance(found, bool) or not isinstance(found, int):
            self.fail(place, "%s, but YAML read %s here" % (need, _kind(found)))
        if minimum is not None and found < minimum:
            self.fail(place, "%s, and %d is less" % (need, found))
        return found

    def texts(self, place, empty=False):
        """A list of texts, each held as `text` is. An empty list only when `empty`."""
        found = self.value(place)
        if not isinstance(found, list):
            self.fail(place, "a list of texts is expected, but %s is here" % _kind(found))
        if not found and not empty:
            self.fail(place, "a list with something in it is expected")
        return [self.text(tuple(place) + (index,)) for index in range(len(found))]

    def table(self, place):
        """A mapping. Its keys are given back as the texts they are written as."""
        found = self.value(place)
        if not isinstance(found, dict):
            self.fail(place, "a mapping of keys and values is expected, but %s is here" % _kind(found))
        return [str(key) for key in found]

    # Holding a mapping to its fields.

    def only(self, place, allowed):
        """Refuse a key that is not in `allowed`, naming where it stands."""
        for key in self.table(place):
            if key not in allowed:
                self.fail(
                    tuple(place) + (key,),
                    "unknown key `%s` in %s; the keys known here are: %s"
                    % (key, self.dotted(tuple(place)), ", ".join("`%s`" % name for name in allowed)),
                    key=True,
                )

    def need(self, place, fields):
        """Refuse a mapping that lacks one of `fields`, naming the field."""
        present = self.table(place)
        for field in fields:
            if field not in present:
                self.fail(place, "lacks the field `%s`" % field)


