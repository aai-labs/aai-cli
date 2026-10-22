use reqwest::Method;
use serde_json::Value;

use crate::{
    cli::{MicrosoftTeamsAction, MicrosoftTeamsCommand},
    config::Context,
    error::AppError,
    http::ApiClient,
};

use super::common::{collection, collection_without_page_size, request, user_root};

pub(super) async fn dispatch(
    client: &ApiClient,
    ctx: &Context,
    command: MicrosoftTeamsCommand,
) -> Result<Value, AppError> {
    match command.action {
        MicrosoftTeamsAction::Get(args) => {
            request(
                client,
                ctx,
                "teams.get",
                Method::GET,
                &format!("teams/{}", enc(&args.team_id)),
                None,
            )
            .await
        }
        MicrosoftTeamsAction::Channels(args) => {
            collection_without_page_size(
                client,
                ctx,
                "teams.channels.list",
                &format!("teams/{}/channels", enc(&args.team_id)),
                args.list.limit,
            )
            .await
        }
        MicrosoftTeamsAction::Channel(args) => {
            request(
                client,
                ctx,
                "teams.channels.get",
                Method::GET,
                &format!(
                    "teams/{}/channels/{}",
                    enc(&args.team_id),
                    enc(&args.channel_id)
                ),
                None,
            )
            .await
        }
        MicrosoftTeamsAction::Members(args) => {
            collection(
                client,
                ctx,
                "teams.members.list",
                &format!("teams/{}/members", enc(&args.team_id)),
                args.list.limit,
            )
            .await
        }
        MicrosoftTeamsAction::Messages(args) => {
            collection(
                client,
                ctx,
                "teams.messages.list",
                &format!(
                    "teams/{}/channels/{}/messages",
                    enc(&args.team_id),
                    enc(&args.channel_id)
                ),
                args.list.limit,
            )
            .await
        }
        MicrosoftTeamsAction::Chats(args) => {
            let root = user_root(ctx, args.user.user_id.as_deref())?;
            collection(
                client,
                ctx,
                "teams.chats.list",
                &format!("{root}/chats"),
                args.list.limit,
            )
            .await
        }
    }
}

fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}
