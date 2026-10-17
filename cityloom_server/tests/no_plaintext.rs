use cityloom_server::accounts::{self, Claims, Repo};
use cityloom_server::crypto::{LookupKey, MasterKeys};
use cityloom_server::db::PgRepo;
use cityloom_server::work::{Id, Kind, Put, WorkRepo};
use sqlx::{Column, PgPool, Row, TypeInfo, ValueRef};

/// Every column of every row of every table of the schema, as the raw bytes Postgres sends.
async fn everything(pool: &PgPool) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for table in ["users", "identities", "sessions", "work"] {
        for row in sqlx::query(sqlx::AssertSqlSafe(format!("select * from {table}"))).fetch_all(pool).await.unwrap() {
            for (i, col) in row.columns().iter().enumerate() {
                let raw = row.try_get_raw(i).unwrap();
                if !ValueRef::is_null(&raw) {
                    out.push((format!("{table}.{} ({})", col.name(), col.type_info().name()), raw.as_bytes().unwrap().to_vec()));
                }
            }
        }
    }
    out
}

#[sqlx::test]
async fn no_row_holds_anything_the_person_signed_in_with(pool: PgPool) {
    let r = PgRepo::new(pool.clone(), MasterKeys::new(&[(1, zeroize::Zeroizing::new([1; 32]))], 1).unwrap(), LookupKey::new(zeroize::Zeroizing::new([3; 32])));
    let claims = Claims {
        provider: "microsoft".into(),
        provider_name: "Microsoft".into(),
        sub: "sub-7f3a9c-very-unique".into(),
        name: Some("Ada Quillfeather".into()),
        preferred_username: Some("adaq_osm_name".into()),
        first_time_name: None,
        label: Some("ada.quillfeather@example.org".into()),
    };
    let (ada, _) = accounts::sign_in(&r, &claims).await.unwrap();
    let mut second = claims.clone();
    second.provider = "apple".into();
    second.sub = "apple-sub-55e1-unique".into();
    second.label = Some("relay.xyz@privaterelay.example".into());
    accounts::link(&r, ada, &second).await.unwrap();
    accounts::set_storage(&r, ada, true).await.unwrap();
    let now = time::OffsetDateTime::now_utc();
    r.create_session([42; 32], ada, now, now + time::Duration::days(30)).await.unwrap();
    r.push(
        ada,
        vec![Put {
            id: Id { kind: Kind::Street, key: "w1~n2~n3".into() },
            base_rev: 0,
            edited_at: now,
            deleted: false,
            body: Some(br#"{"note":"CITYLOOM-MARKER-WORK-BODY"}"#.to_vec()),
        }],
        now,
    )
    .await
    .unwrap();

    let needles = [
        "CITYLOOM-MARKER-WORK-BODY",
        "note",
        "Ada Quillfeather",
        "adaq_osm_name",
        "ada.quillfeather@example.org",
        "sub-7f3a9c-very-unique",
        "microsoft",
        "apple-sub-55e1-unique",
        "apple",
        "relay.xyz@privaterelay.example",
        "true",
    ];
    let rows = everything(&pool).await;
    assert!(rows.len() > 10, "the dump should hold every column: {}", rows.len());
    for (column, bytes) in rows {
        let as_text = String::from_utf8_lossy(&bytes).to_lowercase();
        let hex = hex::encode(&bytes);
        for needle in needles {
            assert!(!as_text.contains(&needle.to_lowercase()), "{column} holds {needle:?}");
            assert!(!hex.contains(&hex::encode(needle)), "{column} holds {needle:?} in hex");
        }
    }
}
