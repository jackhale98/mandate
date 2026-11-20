// The engine's explanation of an item as a tree: each node is a gate, a
// role, a group or a condition, with the engine's verdict (✓ / ✗) and the
// values its evaluator read. Information nodes (`info`) are shown but never
// decide anything. Rendering only: the verdicts come from the engine.
import type { DevNodeView } from "./protocol/DevNodeView";

function mark(n: DevNodeView): { glyph: string; word: string; cls: string } {
  if (n.kind === "info") return { glyph: "i", word: "note", cls: "dev-mark dev-mark-info" };
  return n.ok
    ? { glyph: "✓", word: "holds", cls: "dev-mark dev-mark-ok" }
    : { glyph: "✗", word: "fails", cls: "dev-mark dev-mark-fail" };
}

export function ExplainNode({ node, depth = 0 }: { node: DevNodeView; depth?: number }) {
  const m = mark(node);
  const head = (
    <span className="dev-node-head">
      <span className={m.cls} aria-label={m.word} title={m.word}>
        {m.glyph}
      </span>
      <span className="dev-node-kind">{node.kind}</span>
      <span className="dev-node-label">{node.label}</span>
      {node.detail && <span className="dev-node-detail">{node.detail}</span>}
      {node.values.map((v) => (
        <span className="dev-value" key={v.name}>
          {v.name} = <b>{v.value}</b>
        </span>
      ))}
    </span>
  );
  if (node.children.length === 0) {
    return (
      <li className="dev-node" data-ok={node.ok} data-kind={node.kind}>
        {head}
      </li>
    );
  }
  // Failing branches start open, so the "why not" is on screen.
  return (
    <li className="dev-node" data-ok={node.ok} data-kind={node.kind}>
      <details open={!node.ok || depth < 2}>
        <summary>{head}</summary>
        <ul className="dev-tree">
          {node.children.map((c, i) => (
            <ExplainNode key={`${c.label}-${i}`} node={c} depth={depth + 1} />
          ))}
        </ul>
      </details>
    </li>
  );
}

export function ExplainTree({ tree }: { tree: DevNodeView }) {
  return (
    <ul className="dev-tree dev-tree-root" aria-label="explanation">
      <ExplainNode node={tree} />
    </ul>
  );
}
