import { api } from "../lib/ipc";
import type { TeamDetails } from "../lib/generated";
import { ChannelList, Media, PageFrame } from "./components";
import { pageRequest, type Links, type QueryContext } from "./Workspace";
import { usePage } from "./usePage";

export function TeamView({ name, context, links }: { name: string; context: QueryContext; links: Links }) {
  const query = usePage<TeamDetails>({ ...context, viewKey: `team:${name.toLowerCase()}`, identify: team => team.id,
    load: async (_cursor, refresh) => {
      const team = await api.team({ name, page: pageRequest(context.sessionId, null, refresh) });
      return { ...team.members, items: [team] };
    } });
  const team = query.page?.items[0];
  return <PageFrame query={query} detail empty="Team unavailable">{team && <article>
    <div className="channel-identity"><Media src={team.imageUrl} shape="avatar" retryGeneration={query.imageRetryGeneration} /><h2>{team.displayName}</h2></div>
    <p className="channel-description">{team.description}</p>
    <p>{team.memberCount} members. Open a member to check current live status and watch.</p>
    {team.limited && <p className="notice" role="status">Showing the first 300 members, sorted by login. Twitch does not paginate Teams.</p>}
    {!team.members.items.length && <p>This team has no members.</p>}
    <ChannelList items={team.members.items} retryGeneration={query.imageRetryGeneration} open={links.channel} watch={links.watch} pending={links.pending} />
  </article>}</PageFrame>;
}
