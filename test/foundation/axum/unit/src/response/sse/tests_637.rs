#[cfg(test)]
mod tests {
    use super::*;
    use crate::{routing::get, test_helpers::*, Router};
    use futures_util::stream;
    use serde_json::value::RawValue;
    use std::{collections::HashMap, convert::Infallible};
    use tokio_stream::StreamExt as _;

    #[test]
    fn leading_space_is_not_stripped() {
        let no_leading_space = Event::default().data("\tfoobar");
        assert_eq!(&*no_leading_space.finalize(), b"data: \tfoobar\n\n");

        let leading_space = Event::default().data(" foobar");
        assert_eq!(&*leading_space.finalize(), b"data:  foobar\n\n");
    }

    #[test]
    fn write_data_writer_str() {
        // also confirm that nop writers do nothing :)
        let mut writer = Event::default()
            .into_data_writer()
            .into_event()
            .into_data_writer();
        writer.write_str("").unwrap();
        let mut writer = writer.into_event().into_data_writer();

        writer.write_str("").unwrap();
        writer.write_str("moon ").unwrap();
        writer.write_str("star\nsun").unwrap();
        writer.write_str("").unwrap();
        writer.write_str("set").unwrap();
        writer.write_str("").unwrap();
        writer.write_str(" bye\r").unwrap();

        let event = writer.into_event();

        assert_eq!(
            &*event.finalize(),
            b"data: moon star\ndata: sunset bye\rdata: \n\n"
        );
    }

    #[test]
    fn valid_json_raw_value_chars_handled() {
        let json_string = "{\r\"foo\":  \n\r\r   \"bar\\n\"\n}";
        let json_raw_value_event = Event::default()
            .json_data(serde_json::from_str::<&RawValue>(json_string).unwrap())
            .unwrap();
        assert_eq!(
            &*json_raw_value_event.finalize(),
            b"data: {\rdata: \"foo\":  \ndata: \rdata: \rdata:    \"bar\\n\"\ndata: }\n\n"
        );
    }

    #[crate::test]
    async fn basic() {
        let app = Router::new().route(
            "/",
            get(|| async {
                let stream = stream::iter(vec![
                    Event::default().data("one").comment("this is a comment"),
                    Event::default()
                        .json_data(serde_json::json!({ "foo": "bar" }))
                        .unwrap(),
                    Event::default()
                        .event("three")
                        .retry(Duration::from_secs(30))
                        .id("unique-id"),
                ])
                .map(Ok::<_, Infallible>);
                Sse::new(stream)
            }),
        );

        let client = TestClient::new(app);
        let mut stream = client.get("/").await;

        assert_eq!(stream.headers()["content-type"], "text/event-stream");
        assert_eq!(stream.headers()["cache-control"], "no-cache");

        let event_fields = parse_event(&stream.chunk_text().await.unwrap());
        assert_eq!(event_fields.get("data").unwrap(), "one");
        assert_eq!(event_fields.get("comment").unwrap(), "this is a comment");

        let event_fields = parse_event(&stream.chunk_text().await.unwrap());
        assert_eq!(event_fields.get("data").unwrap(), "{\"foo\":\"bar\"}");
        assert!(!event_fields.contains_key("comment"));

        let event_fields = parse_event(&stream.chunk_text().await.unwrap());
        assert_eq!(event_fields.get("event").unwrap(), "three");
        assert_eq!(event_fields.get("retry").unwrap(), "30000");
        assert_eq!(event_fields.get("id").unwrap(), "unique-id");
        assert!(!event_fields.contains_key("comment"));

        assert!(stream.chunk_text().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn keep_alive() {
        const DELAY: Duration = Duration::from_secs(5);

        let app = Router::new().route(
            "/",
            get(|| async {
                let stream = stream::repeat_with(|| Event::default().data("msg"))
                    .map(Ok::<_, Infallible>)
                    .throttle(DELAY);

                Sse::new(stream).keep_alive(
                    KeepAlive::new()
                        .interval(Duration::from_secs(1))
                        .text("keep-alive-text"),
                )
            }),
        );

        let client = TestClient::new(app);
        let mut stream = client.get("/").await;

        for _ in 0..5 {
            // first message should be an event
            let event_fields = parse_event(&stream.chunk_text().await.unwrap());
            assert_eq!(event_fields.get("data").unwrap(), "msg");

            // then 4 seconds of keep-alive messages
            for _ in 0..4 {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let event_fields = parse_event(&stream.chunk_text().await.unwrap());
                assert_eq!(event_fields.get("comment").unwrap(), "keep-alive-text");
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn keep_alive_ends_when_the_stream_ends() {
        const DELAY: Duration = Duration::from_secs(5);

        let app = Router::new().route(
            "/",
            get(|| async {
                let stream = stream::repeat_with(|| Event::default().data("msg"))
                    .map(Ok::<_, Infallible>)
                    .throttle(DELAY)
                    .take(2);

                Sse::new(stream).keep_alive(
                    KeepAlive::new()
                        .interval(Duration::from_secs(1))
                        .text("keep-alive-text"),
                )
            }),
        );

        let client = TestClient::new(app);
        let mut stream = client.get("/").await;

        // first message should be an event
        let event_fields = parse_event(&stream.chunk_text().await.unwrap());
        assert_eq!(event_fields.get("data").unwrap(), "msg");

        // then 4 seconds of keep-alive messages
        for _ in 0..4 {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let event_fields = parse_event(&stream.chunk_text().await.unwrap());
            assert_eq!(event_fields.get("comment").unwrap(), "keep-alive-text");
        }

        // then the last event
        let event_fields = parse_event(&stream.chunk_text().await.unwrap());
        assert_eq!(event_fields.get("data").unwrap(), "msg");

        // then no more events or keep-alive messages
        assert!(stream.chunk_text().await.is_none());
    }

    fn parse_event(payload: &str) -> HashMap<String, String> {
        let mut fields = HashMap::new();

        let mut lines = payload.lines().peekable();
        while let Some(line) = lines.next() {
            if line.is_empty() {
                assert!(lines.next().is_none());
                break;
            }

            let (mut key, value) = line.split_once(':').unwrap();
            let value = value.trim();
            if key.is_empty() {
                key = "comment";
            }
            fields.insert(key.to_owned(), value.to_owned());
        }

        fields
    }
}
