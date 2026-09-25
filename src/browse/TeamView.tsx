import { useI18n } from "../i18n";
import { api } from "../lib/ipc";
import type { TeamDetails } from "../lib/generated";
import { ChannelList, Media, PageFrame } from "./components";
import { pageRequest, type Links, type QueryContext } from "./Workspace";
import { usePage } from "./usePage";

export function TeamView({ name, context, links }: { name: string; context: QueryContext; links: Links }) {
  const { t, count } = useI18n();
  const query = usePage<TeamDetails>({ ...context, viewKey: `team:${name.toLowerCase()}`, identify: team => team.id,
    load: async (_cursor, refresh) => {
      const team = await api.team({ name, page: pageRequest(context.sessionId, null, refresh) });
      return { ...team.members, items: [team] };
    } });
  const team = query.page?.items[0];
  return <PageFrame query={query} detail empty={t("teams.unavailable")}>{team && <article>
    <div className="channel-identity"><Media src={team.imageUrl} shape="avatar" retryGeneration={query.imageRetryGeneration} /><h2>{team.displayName}</h2></div>
    <p className="channel-description">{team.description}</p>
    <p>{count("teams.memberHint", team.memberCount)}</p>
    {team.limited && <p className="notice" role="status">{t("teams.limitNotice")}</p>}
    {!team.members.items.length && <p>{t("teams.empty")}</p>}
    <ChannelList items={team.members.items} retryGeneration={query.imageRetryGeneration} open={links.channel} watch={links.watch} pending={links.pending} />
  </article>}</PageFrame>;
}
