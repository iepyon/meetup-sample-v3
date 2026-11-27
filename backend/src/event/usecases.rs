use super::errors::{CreateEventError, EventCreatedEvent, ListEventsError};
use super::models::{CreateEventInput, Event, create_event};
use super::repository::EventRepository;
use crate::community::repository::{CommunityMemberRepository, CommunityRepository};
use crate::community::usecases::{Ctx as CommunityCtx, get_community_query};
use crate::shared::event_bus::EventBus;

pub struct Ctx<'a> {
    pub communities: &'a CommunityRepository,
    pub events: &'a EventRepository,
    pub bus: &'a EventBus<EventCreatedEvent>,
    pub members: &'a CommunityMemberRepository,
}

/// コミュニティの存在確認 → ファクトリで生成 → 保存 → EventCreated 発火。
pub async fn create_event_command(
    ctx: &Ctx<'_>,
    input: CreateEventInput,
) -> Result<Event, CreateEventError> {
    ctx.communities
        .find_by_id(&input.community_id)
        .await?
        .ok_or(CreateEventError::CommunityNotFound)?;

    let occurred_at = input.created_at;
    let event = create_event(input)?;
    ctx.events.save(&event).await?;

    ctx.bus.publish(&EventCreatedEvent {
        event_id: event.id.clone(),
        community_id: event.community_id.clone(),
        created_by: event.created_by.clone(),
        title: event.title.clone(),
        occurred_at,
    });
    Ok(event)
}

/// イベント一覧。コミュニティ詳細と同じ可視性規則を適用する:
/// 存在しない・閲覧不可（PRIVATE の非 ACTIVE メンバー）は CommunityNotFound。DRAFT は作成者のみ。
pub async fn list_events_query(
    ctx: &Ctx<'_>,
    community_id: &str,
    viewer_account_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Event>, i64), ListEventsError> {
    let bus = EventBus::default();
    let community_ctx = CommunityCtx {
        communities: ctx.communities,
        members: ctx.members,
        bus: &bus,
    };
    get_community_query(&community_ctx, community_id, viewer_account_id)
        .await
        .map_err(|e| match e {
            crate::community::errors::GetCommunityError::CommunityNotFound => {
                ListEventsError::CommunityNotFound
            }
            crate::community::errors::GetCommunityError::Internal => ListEventsError::Internal,
        })?;
    Ok(ctx
        .events
        .list_by_community(community_id, viewer_account_id, limit, offset)
        .await?)
}
