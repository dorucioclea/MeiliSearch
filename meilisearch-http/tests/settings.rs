use std::convert::Into;
use std::time::Duration;

use assert_json_diff::assert_json_eq;
use async_std::io::prelude::*;
use async_std::task::{block_on, sleep};
use http_service::Body;
use serde_json::json;
use serde_json::Value;

mod common;

// Process:
// - Write a full settings update
// - Delete all settings
// Check:
// - Settings are deleted, all fields are null
// - POST success repond Status Code 202
// - Get success repond Status Code 200
// - Delete success repond Status Code 202
#[test]
fn write_all_and_delete() {
    let mut server = common::setup_server().unwrap();

    // 1 - Create the index

    let body = json!({
        "uid": "movies",
    })
    .to_string()
    .into_bytes();

    let req = http::Request::post("/indexes")
        .body(Body::from(body))
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 201);

    // 2 - Send the settings

    let json = json!({
        "rankingRules": [
            "_typo",
            "_words",
            "_proximity",
            "_attribute",
            "_words_position",
            "_exact",
            "dsc(release_date)",
            "dsc(rank)",
        ],
        "rankingDistinct": "movie_id",
        "identifier": "uid",
        "searchableAttributes": [
            "uid",
            "movie_id",
            "title",
            "description",
            "poster",
            "release_date",
            "rank",
        ],
        "displayedAttributes": [
            "title",
            "description",
            "poster",
            "release_date",
            "rank",
        ],
        "stopWords": [
            "the",
            "a",
            "an",
        ],
        "synonyms": {
            "wolverine": ["xmen", "logan"],
            "logan": ["wolverine"],
        },
        "indexNewFields": false,
    });

    let body = json.to_string().into_bytes();

    let req = http::Request::post("/indexes/movies/settings")
        .body(Body::from(body))
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 202);

    block_on(sleep(Duration::from_secs(1)));

    // 3 - Get all settings and compare to the previous one

    let req = http::Request::get("/indexes/movies/settings")
        .body(Body::empty())
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 200);

    let mut buf = Vec::new();
    block_on(res.into_body().read_to_end(&mut buf)).unwrap();
    let res_value: Value = serde_json::from_slice(&buf).unwrap();

    assert_json_eq!(json, res_value, ordered: false);

    // 4 - Delete all settings

    let req = http::Request::delete("/indexes/movies/settings")
        .body(Body::empty())
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 202);

    block_on(sleep(Duration::from_secs(2)));

    // 5 - Get all settings and check if they are empty

    let req = http::Request::get("/indexes/movies/settings")
        .body(Body::empty())
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 200);

    let mut buf = Vec::new();
    block_on(res.into_body().read_to_end(&mut buf)).unwrap();
    let res_value: Value = serde_json::from_slice(&buf).unwrap();

    let json = json!({
        "rankingRules": null,
        "rankingDistinct": null,
        "identifier": null,
        "searchableAttributes": null,
        "displayedAttributes": null,
        "stopWords": null,
        "synonyms": null,
        "indexNewFields": null,
    });

    assert_json_eq!(json, res_value, ordered: false);
}

// Process:
// - Write a full setting update
// - Rewrite an other settings confirmation
// Check:
// - Settings are overwrited
// - Forgotten attributes are deleted
// - Null attributes are deleted
// - Empty attribute are deleted
#[test]
fn write_all_and_update() {
    let mut server = common::setup_server().unwrap();

    // 1 - Create the index

    let body = json!({
        "uid": "movies",
    })
    .to_string()
    .into_bytes();

    let req = http::Request::post("/indexes")
        .body(Body::from(body))
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 201);

    // 2 - Send the settings

    let json = json!({
        "rankingRules": [
            "_typo",
            "_words",
            "_proximity",
            "_attribute",
            "_words_position",
            "_exact",
            "dsc(release_date)",
            "dsc(rank)",
        ],
        "rankingDistinct": "movie_id",
        "identifier": "uid",
        "searchableAttributes": [
            "uid",
            "movie_id",
            "title",
            "description",
            "poster",
            "release_date",
            "rank",
        ],
        "displayedAttributes": [
            "title",
            "description",
            "poster",
            "release_date",
            "rank",
        ],
        "stopWords": [
            "the",
            "a",
            "an",
        ],
        "synonyms": {
            "wolverine": ["xmen", "logan"],
            "logan": ["wolverine"],
        },
        "indexNewFields": false,
    });

    let body = json.to_string().into_bytes();

    let req = http::Request::post("/indexes/movies/settings")
        .body(Body::from(body))
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 202);

    block_on(sleep(Duration::from_secs(1)));

    // 3 - Get all settings and compare to the previous one

    let req = http::Request::get("/indexes/movies/settings")
        .body(Body::empty())
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 200);

    let mut buf = Vec::new();
    block_on(res.into_body().read_to_end(&mut buf)).unwrap();
    let res_value: Value = serde_json::from_slice(&buf).unwrap();

    assert_json_eq!(json, res_value, ordered: false);

    // 4 - Update all settings

    let json_update = json!({
        "rankingRules": [
            "_typo",
            "_words",
            "_proximity",
            "_attribute",
            "_words_position",
            "_exact",
            "dsc(release_date)",
        ],
        "identifier": "uid",
        "searchableAttributes": [
            "title",
            "description",
            "uid",
        ],
        "displayedAttributes": [
            "title",
            "description",
            "release_date",
            "rank",
            "poster",
        ],
        "stopWords": [
        ],
        "synonyms": {
            "wolverine": ["xmen", "logan"],
            "logan": ["wolverine", "xmen"],
        },
        "indexNewFields": false,
    });

    let body_update = json_update.to_string().into_bytes();

    let req = http::Request::post("/indexes/movies/settings")
        .body(Body::from(body_update))
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 202);

    block_on(sleep(Duration::from_secs(1)));

    // 5 - Get all settings and check if the content is the same of (4)

    let req = http::Request::get("/indexes/movies/settings")
        .body(Body::empty())
        .unwrap();
    let res = server.simulate(req).unwrap();
    assert_eq!(res.status(), 200);

    let mut buf = Vec::new();
    block_on(res.into_body().read_to_end(&mut buf)).unwrap();
    let res_value: Value = serde_json::from_slice(&buf).unwrap();

    let res_expected = json!({
        "rankingRules": [
            "_typo",
            "_words",
            "_proximity",
            "_attribute",
            "_words_position",
            "_exact",
            "dsc(release_date)",
        ],
        "rankingDistinct": null,
        "identifier": "uid",
        "searchableAttributes": [
            "title",
            "description",
            "uid",
        ],
        "displayedAttributes": [
            "title",
            "description",
            "release_date",
            "rank",
            "poster",
        ],
        "stopWords": null,
        "synonyms": {
            "wolverine": ["xmen", "logan"],
            "logan": ["wolverine", "xmen"],
        },
        "indexNewFields": false
    });

    assert_json_eq!(res_expected, res_value, ordered: false);
}
