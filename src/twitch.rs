use anyhow::{Context, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest::Client;

const TWITCH_HOMEPAGE: &str = "https://www.twitch.tv";
const TWITCH_API_GQL: &str = "https://gql.twitch.tv/gql";
const TWITCH_API_USHER: &str = "https://usher.ttvnw.net/api/channel/hls/";
pub const TWITCH_CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

pub async fn extract_client_id(http_client: &Client) -> Result<String> {
    let response = http_client.get(TWITCH_HOMEPAGE).send().await?;

    let content = response.text().await?;
    let index = content
        .find("Client-ID")
        .context("Can't find Client-ID in page")?;
    Ok(content[index..index + 42]
        .strip_prefix("Client-ID\":\"")
        .unwrap()
        .to_string())
}

#[derive(GraphQLQuery)]
#[graphql(
schema_path = "src/schema.graphql",
query_path = "src/query.graphql",
response_derives = "Debug"
)]
pub struct MainQuery;

type Cursor = String;

pub async fn execute_main_query(
    http_client: &Client,
    client_id: &str,
    as_user: &str,
    first: Option<i64>,
    after: Option<Cursor>,
) -> Result<Response<main_query::ResponseData>> {
    let query = MainQuery::build_query(main_query::Variables {
        as_user: Some(as_user.to_string()),
        first,
        after,
    });

    let response = http_client
        .post(TWITCH_API_GQL)
        .header("Client-Id", client_id)
        .json(&query)
        .send()
        .await?;

    Ok(response.json().await?)
}

#[derive(Debug)]
pub struct StreamPlaybackToken {
    token: String,
    sig: String,
}

pub async fn get_stream_playback_token(
    http_client: &Client,
    client_id: &str,
    channel: &str,
) -> Result<StreamPlaybackToken> {
    let part0 = "{\n\t\"operationName\": \"PlaybackAccessToken\",\n\t\"extensions\": {\n\t\t\"persistedQuery\": {\n\t\t\t\"version\": 1,\n\t\t\t\"sha256Hash\": \"0828119ded1c13477966434e15800ff57ddacf13ba1911c129dc2200705b0712\"\n\t\t}\n\t},\n\t\"variables\": {\n\t\t\"isLive\": true,\n\t\t\"login\": \"";
    let part1 =
        "\",\n\t\t\"isVod\": false,\n\t\t\"vodID\": \"\",\n\t\t\"playerType\": \"embed\"\n\t}\n}";
    let query = format!("{}{}{}", part0, channel, part1);

    let response = http_client
        .post(TWITCH_API_GQL)
        .header("Client-Id", client_id)
        .body(query)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let value: serde_json::Value = response.json().await?;
    let values = value.as_object().unwrap()["data"]
        .as_object().unwrap()["streamPlaybackAccessToken"]
        .as_object().unwrap();
    Ok(StreamPlaybackToken {
        token: values["value"].as_str().unwrap().to_string(),
        sig: values["signature"].as_str().unwrap().to_string(),
    })
}

pub async fn usher_get_hls_playlist(
    http_client: &Client,
    channel: &str,
    token: &StreamPlaybackToken,
) -> Result<String> {
    let url = format!("{}{}.m3u8", TWITCH_API_USHER, channel);
    let response = http_client
        .get(&url)
        .query(&[
            ("player", "twitchweb"),
            ("p", "975642"),
            ("type", "any"),
            ("allow_source", "true"),
            ("token", &token.token),
            ("sig", &token.sig),
        ])
        .send()
        .await?;
    Ok(response.text().await?)
}
