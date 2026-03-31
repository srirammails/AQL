# aql_parser/parser.py
"""AQL v0.5 Parser using Lark"""

from pathlib import Path
from lark import Lark, Transformer, v_args
from lark.exceptions import UnexpectedInput, UnexpectedEOF

from .types import (
    Verb, MemoryType, Comparator, ScopeValue, AggOp, WindowType,
    Condition, KeyExpr, Predicate, Payload,
    ReturnMod, LimitMod, WeightMod, ThresholdMod,
    TimeoutMod, OrderMod, ConfidenceMod, SourceMod,
    ScopeMod, NamespaceMod, TtlMod,
    WindowMod, AggregateFunc, AggregateMod, HavingMod,
    ReflectSource, ExecutionPlan
)
from .errors import AqlError

GRAMMAR_PATH = Path(__file__).parent.parent / "grammar" / "aql.lark"


class AqlTransformer(Transformer):
    """Transform Lark parse tree into ExecutionPlan."""

    # ── Terminals ───────────────────────────────────────────────────────
    def CNAME(self, tok):
        return str(tok)

    def INT(self, tok):
        return int(tok)

    def DECIMAL(self, tok):
        return float(tok)

    def ESCAPED_STRING(self, tok):
        return str(tok)[1:-1]  # strip quotes

    # ── Primitives ──────────────────────────────────────────────────────
    def identifier(self, items):
        return items[0]

    def field(self, items):
        return ".".join(str(i) for i in items)

    def true_val(self, items):
        return True

    def false_val(self, items):
        return False

    def boolean(self, items):
        return items[0]

    def embedding_ref(self, items):
        return {"type": "embedding_ref", "name": items[0]}

    def variable(self, items):
        return {"type": "variable", "name": items[0]}

    def str_val(self, items):
        return items[0]

    def int_val(self, items):
        return items[0]

    def decimal_val(self, items):
        return items[0]

    def bool_val(self, items):
        return items[0]

    def embed_val(self, items):
        return items[0]

    def var_val(self, items):
        return items[0]

    def expression(self, items):
        return items[0]

    def value(self, items):
        return items[0]

    # ── Duration ────────────────────────────────────────────────────────
    def ms_unit(self, items):
        return "ms"

    def s_unit(self, items):
        return "s"

    def m_unit(self, items):
        return "m"

    def h_unit(self, items):
        return "h"

    def d_unit(self, items):
        return "d"

    def time_unit(self, items):
        return items[0]

    def duration(self, items):
        return TimeoutMod(value=items[0], unit=items[1])

    # ── Comparators ─────────────────────────────────────────────────────
    def eq_op(self, items):
        return Comparator.EQ

    def neq_op(self, items):
        return Comparator.NEQ

    def gt_op(self, items):
        return Comparator.GT

    def lt_op(self, items):
        return Comparator.LT

    def gte_op(self, items):
        return Comparator.GTE

    def lte_op(self, items):
        return Comparator.LTE

    def comparator(self, items):
        return items[0]

    # ── Conditions ──────────────────────────────────────────────────────
    def simple_cond(self, items):
        return Condition(
            field=items[0],
            op=items[1],
            value=items[2]
        )

    def paren_cond(self, items):
        return items[0]

    def and_condition(self, items):
        if len(items) == 1:
            return items[0]
        # Chain AND conditions
        result = items[0]
        for item in items[1:]:
            result = Condition(
                field="", op=Comparator.EQ, value=None,
                left=result, right=item, logical_op="AND"
            )
        return result

    def or_condition(self, items):
        if len(items) == 1:
            return items[0]
        # Chain OR conditions
        result = items[0]
        for item in items[1:]:
            result = Condition(
                field="", op=Comparator.EQ, value=None,
                left=result, right=item, logical_op="OR"
            )
        return result

    # ── Key Expression ──────────────────────────────────────────────────
    def key_expr(self, items):
        return KeyExpr(field=items[0], value=items[1])

    # ── Predicates ──────────────────────────────────────────────────────
    def key_pred(self, items):
        return Predicate(type="key", key_expr=items[0])

    def where_pred(self, items):
        return Predicate(type="where", condition=items[0])

    def like_pred(self, items):
        return Predicate(type="like", expression=items[0])

    def pattern_pred(self, items):
        return Predicate(type="pattern", expression=items[0])

    def all_pred(self, items):
        return Predicate(type="all")

    def exact_pred(self, items):
        return items[0]

    def similarity_pred(self, items):
        return items[0]

    def scan_pred(self, items):
        return items[0]

    # ── Window Predicates ─────────────────────────────────────────────────
    def window_last_n(self, items):
        return WindowMod(window_type=WindowType.LAST_N, count=items[0])

    def window_last_dur(self, items):
        dur = items[0]  # TimeoutMod from duration
        return WindowMod(
            window_type=WindowType.LAST_DUR,
            duration_value=dur.value,
            duration_unit=dur.unit
        )

    def window_top(self, items):
        return WindowMod(
            window_type=WindowType.TOP,
            count=items[0],
            field=items[1]
        )

    def window_since(self, items):
        return WindowMod(window_type=WindowType.SINCE, key_expr=items[0])

    def window_type(self, items):
        return items[0]

    def window_pred(self, items):
        window_mod = items[0]
        return Predicate(type="window", window=window_mod)

    def predicate(self, items):
        return items[0]

    # ── Memory Types ────────────────────────────────────────────────────
    def episodic_type(self, items):
        return MemoryType.EPISODIC

    def semantic_type(self, items):
        return MemoryType.SEMANTIC

    def procedural_type(self, items):
        return MemoryType.PROCEDURAL

    def working_type(self, items):
        return MemoryType.WORKING

    def tools_type(self, items):
        return MemoryType.TOOLS

    def memory_type(self, items):
        return items[0]

    # ── Verbs ───────────────────────────────────────────────────────────
    def lookup_verb(self, items):
        return Verb.LOOKUP

    def recall_verb(self, items):
        return Verb.RECALL

    def scan_verb(self, items):
        return Verb.SCAN

    def load_verb(self, items):
        return Verb.LOAD

    def verb(self, items):
        return items[0]

    # ── Modifiers ───────────────────────────────────────────────────────
    def field_list(self, items):
        return [str(i) for i in items]

    def return_mod(self, items):
        return ReturnMod(fields=items[0])

    def limit_mod(self, items):
        return LimitMod(value=items[0])

    def weight_mod(self, items):
        return WeightMod(field=items[0])

    def threshold_mod(self, items):
        return ThresholdMod(value=items[0])

    def timeout_mod(self, items):
        return items[0]  # already TimeoutMod from duration

    def asc_dir(self, items):
        return "ASC"

    def desc_dir(self, items):
        return "DESC"

    def sort_dir(self, items):
        return items[0]

    def order_mod(self, items):
        field = items[0]
        direction = items[1] if len(items) > 1 else "ASC"
        return OrderMod(field=field, direction=direction)

    def confidence_mod(self, items):
        return ConfidenceMod(value=items[0])

    def identifier_list(self, items):
        return [str(i) for i in items]

    def source_mod(self, items):
        return SourceMod(sources=items[0])

    # ── Aggregate Modifiers ───────────────────────────────────────────────
    def count_op(self, items):
        return AggOp.COUNT

    def avg_op(self, items):
        return AggOp.AVG

    def sum_op(self, items):
        return AggOp.SUM

    def min_op(self, items):
        return AggOp.MIN

    def max_op(self, items):
        return AggOp.MAX

    def agg_op(self, items):
        return items[0]

    def agg_field_func(self, items):
        return AggregateFunc(op=items[0], field=items[1], alias=items[2])

    def agg_count_star(self, items):
        return AggregateFunc(op=AggOp.COUNT, field=None, alias=items[0])

    def aggregate_func(self, items):
        return items[0]

    def aggregate_mod(self, items):
        return AggregateMod(functions=list(items))

    def having_mod(self, items):
        return HavingMod(condition=items[0])

    def modifier(self, items):
        return items[0]

    # ── Scope/Namespace/TTL ─────────────────────────────────────────────
    def private_scope(self, items):
        return ScopeValue.PRIVATE

    def shared_scope(self, items):
        return ScopeValue.SHARED

    def cluster_scope(self, items):
        return ScopeValue.CLUSTER

    def scope_value(self, items):
        return items[0]

    def scope_mod(self, items):
        return ScopeMod(value=items[0])

    def namespace_mod(self, items):
        return NamespaceMod(value=items[0])

    def ttl_mod(self, items):
        t = items[0]  # TimeoutMod
        return TtlMod(value=t.value, unit=t.unit)

    # ── Payload ─────────────────────────────────────────────────────────
    def field_value(self, items):
        return (items[0], items[1])

    def field_value_list(self, items):
        return dict(items)

    def payload(self, items):
        return Payload(fields=items[0])

    # ── Write Statements ────────────────────────────────────────────────
    def store_stmt(self, items):
        it = iter(items)
        memory_type = next(it)
        payload = next(it)
        scope = namespace = ttl = None
        for item in it:
            if isinstance(item, ScopeMod):
                scope = item
            elif isinstance(item, NamespaceMod):
                namespace = item
            elif isinstance(item, TtlMod):
                ttl = item
        return ExecutionPlan(
            verb=Verb.STORE,
            memory_type=memory_type,
            payload=payload,
            scope=scope,
            namespace=namespace,
            ttl=ttl,
        )

    def update_stmt(self, items):
        return ExecutionPlan(
            verb=Verb.UPDATE,
            memory_type=items[0],
            predicate=Predicate(type="where", condition=items[1]),
            payload=items[2],
        )

    def link_stmt(self, items):
        return ExecutionPlan(
            verb=Verb.LINK,
            memory_type=items[0],
            link_from=items[1],
            link_to_type=items[2],
            link_to=items[3],
        )

    def write_stmt(self, items):
        return items[0]

    # ── Read Statements ─────────────────────────────────────────────────
    def read_stmt(self, items):
        it = iter(items)
        verb = next(it)
        memory_type = None
        predicate = None
        modifiers = []
        for item in it:
            if isinstance(item, MemoryType):
                memory_type = item
            elif isinstance(item, Predicate):
                predicate = item
            else:
                modifiers.append(item)
        return ExecutionPlan(
            verb=verb,
            memory_type=memory_type,
            predicate=predicate,
            modifiers=modifiers,
        )

    # ── Forget Statement ────────────────────────────────────────────────
    def forget_stmt(self, items):
        return ExecutionPlan(
            verb=Verb.FORGET,
            memory_type=items[0],
            predicate=items[1],
        )

    # ── Reflect Statement ───────────────────────────────────────────────
    def context_expr(self, items):
        return items[0]

    def reflect_source(self, items):
        memory_type = items[0]
        predicate = items[1] if len(items) > 1 else None
        return ReflectSource(memory_type=memory_type, predicate=predicate)

    def then_clause(self, items):
        return items[0]

    def reflect_stmt(self, items):
        it = iter(items)
        context_expr = next(it)
        sources = []
        modifiers = []
        then_stmt = None
        for item in it:
            if isinstance(item, ReflectSource):
                sources.append(item)
            elif isinstance(item, ExecutionPlan):
                then_stmt = item
            else:
                modifiers.append(item)
        return ExecutionPlan(
            verb=Verb.REFLECT,
            context_expr=context_expr,
            sources=sources,
            modifiers=modifiers,
            then_stmt=then_stmt,
        )

    # ── Pipeline Statement ──────────────────────────────────────────────
    def stage_stmt(self, items):
        return items[0]

    def pipeline_stage(self, items):
        return items[0]

    def pipeline_stmt(self, items):
        it = iter(items)
        first = next(it)

        # Check if first is identifier (name) or duration (timeout)
        name = None
        timeout = None

        if isinstance(first, str):
            name = first
            timeout = next(it)
        else:
            timeout = first

        stages = list(it)

        return ExecutionPlan(
            verb=Verb.PIPELINE,
            pipeline_name=name,
            timeout=timeout,
            stages=stages,
        )

    # ── Top Level ───────────────────────────────────────────────────────
    def statement(self, items):
        return items[0]

    def program(self, items):
        return items if len(items) > 1 else items[0]


# Load grammar and create parser
_parser = Lark(
    GRAMMAR_PATH.read_text(),
    parser="earley",
    start="program",
    ambiguity="resolve",
)

_transformer = AqlTransformer()


def parse(query: str) -> ExecutionPlan:
    """
    Parse an AQL query string and return an ExecutionPlan.

    Args:
        query: The AQL query string to parse

    Returns:
        ExecutionPlan representing the parsed query

    Raises:
        AqlError: If the query is invalid or cannot be parsed
    """
    query = query.strip()
    if not query:
        raise AqlError("Empty query")
    try:
        tree = _parser.parse(query)
        return _transformer.transform(tree)
    except UnexpectedEOF as e:
        raise AqlError(f"Unexpected end of query: {e}") from e
    except UnexpectedInput as e:
        raise AqlError(f"Parse error at position {e.pos_in_stream}: {e}") from e
    except Exception as e:
        raise AqlError(f"Parse failed: {e}") from e
