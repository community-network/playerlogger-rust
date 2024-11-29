use bson::Document;
use chrono::Local;
use futures::StreamExt;
use mongodb::{options::IndexOptions, Client, Collection, IndexModel};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(_) => println!(".env not found, using env variables..."),
    };

    let mongo_url = env::var("MONGO_DETAILS_STRING").expect("MONGO_DETAILS_STRING wasn't set"); //
                                                                                                // Try connect to mongo client
    let client = Client::with_uri_str(mongo_url).await?;
    let db = client.database("serverManager");
    let player_col: Collection<Document> = db.collection("communityPlayers");
    let log_db = client.database("playerLogger");

    let mut watch = player_col
        .watch([bson::doc! {"$match": {"operationType": "update"}}], None)
        .await?;

    while let Some(next_task) = watch.next().await {
        match next_task {
            Ok(result) => {
                // check if players db item changed
                let has_player = match result.update_description {
                    Some(update_description) => {
                        match update_description.updated_fields.get_array("players") {
                            Ok(_) => true,
                            Err(_) => false,
                        }
                    }
                    None => false,
                };

                if has_player {
                    match result.document_key {
                        Some(document) => match document.clone().get_str("_id") {
                            Ok(document_id) => {
                                let current_data = player_col
                                    .find_one(bson::doc! { "_id": document_id }, None)
                                    .await?;
                                let log_server: Collection<Document> =
                                    log_db.collection(document_id);

                                // update log with new data
                                match current_data {
                                    Some(data) => {
                                        log_server
                                            .insert_one(
                                                bson::doc! {
                                                    "players": data.get_array("players")?,
                                                    "spectators": data.get_array("spectators")?,
                                                    "createdAt": Local::now()
                                                },
                                                None,
                                            )
                                            .await?;
                                        let options = IndexOptions::builder()
                                            .unique(true)
                                            .expire_after(Some(Duration::new(86400, 0)))
                                            .build();
                                        let model = IndexModel::builder()
                                            .keys(bson::doc! {"createdAt": 1})
                                            .options(options)
                                            .build();
                                        log_server.create_index(model, None).await?
                                    }
                                    None => continue,
                                }
                            }
                            Err(_) => continue,
                        },
                        None => continue,
                    };
                }
            }
            Err(_) => continue,
        }
    }
    Ok(())
}
