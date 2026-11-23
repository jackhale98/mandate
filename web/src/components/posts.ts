// Constitution posts in words (DEC-098): every label comes from the
// constitution's Fluent keys, so no office or body is named here.
import type { Translator } from "../i18n/fluent";
import type { PostDto } from "../engine/protocol/PostDto";
import type { BodyView } from "../engine/protocol/BodyView";
import type { MemberView } from "../engine/protocol/MemberView";
import type { OfficeView } from "../engine/protocol/OfficeView";
import type { SignerView } from "../engine/protocol/SignerView";

/** A post as on the ballot: a districted seat by its number, any other post by its label. */
export function postTitle(tr: Translator, p: PostDto): string {
  return p.constituency_number !== null ? districtLabel(tr, p.constituency_number, p.at.tier) : tr.t(p.label_key);
}

/** A district by its number and tier, in the constitution's words ("Ward 3" for the local tier). */
export function districtLabel(tr: Translator, n: number, tier: string): string {
  return tr.t("ui-post-seat", { n, tier });
}

/** The body bills are voted in (the procedure's first vote), if the view has one. */
export function legislature(o: OfficeView | null | undefined): BodyView | null {
  return o?.bodies.find((b) => b.legislature) ?? null;
}

/** Everyone in office here: the executive, then each body's members, in order. */
export function officials(o: OfficeView | null | undefined): MemberView[] {
  if (!o) return [];
  return (o.executive ? [o.executive] : []).concat(o.bodies.flatMap((b) => b.members));
}

/** Who signs a bill and how they lean (`ui-signer-lean`), e.g. the office's label: "will sign". */
export function signerText(tr: Translator, s: SignerView): string {
  return tr.t("ui-signer-lean", { office: tr.t(s.office.label_key), lean: tr.t(`signer-${s.lean}`) });
}
