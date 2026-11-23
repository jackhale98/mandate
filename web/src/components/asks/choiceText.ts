// Words for a system's own choice in a conversation (talk hooks, S4): an
// ask's "Put in a word for { $asker }". The engine sends the label and the
// player's line as Fluent keys with arguments (`name_args`; `*_key`
// arguments are keys); an ordinary topic has none.
import type { TopicView } from "../../engine/protocol/TopicView";
import type { Translator } from "../../i18n/fluent";

/** The button: a topic's name, or a choice's label with its arguments. */
export function topicLabel(tr: Translator, t: TopicView): string {
  return t.name_args ? tr.event(t.name_key, t.name_args) : tr.t(t.name_key);
}

/** What the player says: a choice's own line, else the topic's name. */
export function topicSay(tr: Translator, t: TopicView): string {
  return t.say_key && t.name_args ? tr.event(t.say_key, t.name_args) : topicLabel(tr, t);
}
