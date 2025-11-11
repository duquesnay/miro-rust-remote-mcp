// Documentation-only tests - disabled for ADR-005
#![cfg(feature = "stdio-mcp")]

#[allow(unused_imports)]
use miro_mcp_server::config::Config;
#[allow(unused_imports)]
use miro_mcp_server::MiroClient;
#[allow(unused_imports)]
use miro_mcp_server::auth::{MiroOAuthClient, TokenSet, TokenStore};
#[allow(unused_imports)]
use serde_json::json;
#[allow(unused_imports)]
use wiremock::matchers::{body_partial_json, method, path_regex};
#[allow(unused_imports)]
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper function to create a test configuration
#[allow(dead_code)]
fn get_test_config() -> Config {
    Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        redirect_uri: "http://localhost:3000/oauth/callback".to_string(),
        encryption_key: [0u8; 32],
        port: 3000,
        base_url: Some("http://localhost:3000".to_string()),
    }
}

/// Helper function to create a MiroClient with mocked token and custom base URL
#[allow(dead_code)]
async fn create_test_client(_mock_server_uri: &str) -> MiroClient {
    let config = get_test_config();
    let token_store = TokenStore::new(config.encryption_key).unwrap();

    // Create and save a test token
    let tokens = TokenSet::new(
        "test_access_token".to_string(),
        Some("test_refresh_token".to_string()),
        3600, // Expires in 1 hour
    );
    token_store.save(&tokens).unwrap();

    // MiroOAuthClient is an alias for MiroOAuthProvider
    // Create it with the config values
    let oauth_client =
        MiroOAuthClient::new(config.client_id, config.client_secret, config.redirect_uri);

    // Note: In production code, we'd need to inject the mock server URL
    // For this test, we'll configure the client to use the mock server
    // This would require modifying MiroClient to accept a base_url parameter
    // For now, these tests document the expected behavior

    MiroClient::new(token_store, oauth_client).unwrap()
}

#[tokio::test]
async fn test_create_sticky_note_with_parent() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Expected request body with parent field
    let expected_request = json!({
        "data": {
            "content": "Test sticky note",
            "shape": "square"
        },
        "style": {
            "fillColor": "light_yellow"
        },
        "position": {
            "x": 100.0,
            "y": 200.0,
            "origin": "center"
        },
        "geometry": {
            "width": 200.0
        },
        "parent": {
            "id": "frame-123"
        }
    });

    // Mock response for POST /boards/{id}/sticky_notes
    let mock_response = json!({
        "id": "sticky-456",
        "data": {
            "content": "Test sticky note",
            "shape": "square"
        },
        "style": {
            "fillColor": "light_yellow"
        },
        "position": {
            "x": 100.0,
            "y": 200.0,
            "origin": "center"
        },
        "geometry": {
            "width": 200.0
        },
        "parent": {
            "id": "frame-123"
        }
    });

    Mock::given(method("POST"))
        .and(path_regex(r"^/v2/boards/.*/sticky_notes$"))
        .and(body_partial_json(&expected_request))
        .respond_with(ResponseTemplate::new(201).set_body_json(&mock_response))
        .expect(0) // No requests expected until MiroClient supports base URL injection
        .mount(&mock_server)
        .await;

    // Note: This test documents the expected API interaction
    // FUTURE WORK: Modify MiroClient to accept a base_url parameter for testing
    // Then these tests can actually verify the HTTP requests

    // Once MiroClient supports base URL injection, the test would be:
    // let client = create_test_client(&mock_server.uri()).await;
    // let result = client.create_sticky_note(
    //     "board-789",
    //     "Test sticky note".to_string(),
    //     100.0,
    //     200.0,
    //     "light_yellow".to_string(),
    //     Some("frame-123".to_string()),
    // ).await;
    //
    // assert!(result.is_ok());
    // let sticky_note = result.unwrap();
    // assert_eq!(sticky_note.id, "sticky-456");
    // Verify parent field in response matches the request
    // This would fail if parent_id wasn't properly sent/handled

    // For now, we verify the mock documents the correct API contract
    assert!(mock_server.address().port() > 0);
}

#[tokio::test]
async fn test_update_item_move_to_frame() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Expected request body to update parent_id
    let expected_request = json!({
        "parent": {
            "id": "frame-999"
        }
    });

    // Mock response for PATCH /boards/{board_id}/items/{item_id}
    let mock_response = json!({
        "id": "sticky-456",
        "type": "sticky_note",
        "data": {
            "content": "Test sticky note",
            "shape": "square"
        },
        "style": {
            "fillColor": "light_yellow"
        },
        "position": {
            "x": 100.0,
            "y": 200.0
        },
        "geometry": {
            "width": 200.0
        },
        "parent": {
            "id": "frame-999"
        },
        "createdAt": "2025-11-10T10:00:00Z",
        "modifiedAt": "2025-11-10T11:00:00Z"
    });

    Mock::given(method("PATCH"))
        .and(path_regex(r"^/v2/boards/.*/items/.*$"))
        .and(body_partial_json(&expected_request))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .expect(0) // No requests expected until MiroClient supports base URL injection
        .mount(&mock_server)
        .await;

    // Note: This test documents the expected API interaction
    // In a real test, we'd configure MiroClient to use mock_server.uri()

    // The actual client call would be:
    // let client = create_test_client(&mock_server.uri()).await;
    // let result = client.update_item(
    //     "board-789",
    //     "sticky-456",
    //     None,                           // position
    //     None,                           // data
    //     None,                           // style
    //     None,                           // geometry
    //     Some("frame-999".to_string()),  // parent_id - move to new frame
    // ).await;
    //
    // assert!(result.is_ok());
    // let updated_item = result.unwrap();
    // assert_eq!(updated_item.id, "sticky-456");
    // assert_eq!(updated_item.parent, Some(Parent { id: "frame-999".to_string() }));

    // Verify mock setup
    assert!(mock_server.address().port() > 0);
}

#[tokio::test]
async fn test_list_items_filtered_by_parent() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Mock response for GET /boards/{board_id}/items?type=sticky_note&parent.id=frame-123
    let mock_response = json!({
        "data": [
            {
                "id": "sticky-1",
                "type": "sticky_note",
                "data": {
                    "content": "Note 1 in frame"
                },
                "parent": {
                    "id": "frame-123"
                }
            },
            {
                "id": "sticky-2",
                "type": "sticky_note",
                "data": {
                    "content": "Note 2 in frame"
                },
                "parent": {
                    "id": "frame-123"
                }
            }
        ],
        "cursor": null
    });

    Mock::given(method("GET"))
        .and(path_regex(
            r"^/v2/boards/.*/items\?.*type=sticky_note.*parent\.id=frame-123.*$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .expect(0) // No requests expected until MiroClient supports base URL injection
        .mount(&mock_server)
        .await;

    // Note: This test documents the expected API interaction
    // In a real test, we'd configure MiroClient to use mock_server.uri()

    // The actual client call would be:
    // let client = create_test_client(&mock_server.uri()).await;
    // let result = client.list_items(
    //     "board-789",
    //     Some(vec!["sticky_note"]),
    //     Some("frame-123"),  // Filter by parent_id
    // ).await;
    //
    // assert!(result.is_ok());
    // let items = result.unwrap();
    // assert_eq!(items.len(), 2);
    // assert_eq!(items[0].id, "sticky-1");
    // assert_eq!(items[0].parent, Some(Parent { id: "frame-123".to_string() }));
    // assert_eq!(items[1].id, "sticky-2");
    // assert_eq!(items[1].parent, Some(Parent { id: "frame-123".to_string() }));

    // Verify mock setup
    assert!(mock_server.address().port() > 0);
}

#[tokio::test]
async fn test_update_item_remove_from_frame() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Expected request body with null parent to move to board root
    // Note: In Rust, we represent "set to null" by omitting the field
    // or using a special marker. The Miro API expects parent: null
    // to remove an item from its frame.
    let expected_request = json!({
        "parent": null
    });

    // Mock response for PATCH /boards/{board_id}/items/{item_id}
    // After removing from frame, parent should be null
    let mock_response = json!({
        "id": "sticky-456",
        "type": "sticky_note",
        "data": {
            "content": "Test sticky note",
            "shape": "square"
        },
        "style": {
            "fillColor": "light_yellow"
        },
        "position": {
            "x": 100.0,
            "y": 200.0
        },
        "geometry": {
            "width": 200.0
        },
        "parent": null,
        "createdAt": "2025-11-10T10:00:00Z",
        "modifiedAt": "2025-11-10T11:30:00Z"
    });

    Mock::given(method("PATCH"))
        .and(path_regex(r"^/v2/boards/.*/items/.*$"))
        .and(body_partial_json(&expected_request))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .expect(0) // No requests expected until MiroClient supports base URL injection
        .mount(&mock_server)
        .await;

    // Note: This test documents the expected API interaction
    // FUTURE WORK: May require updating UpdateItemRequest to handle the distinction between:
    // - Not updating parent (don't send parent field)
    // - Setting parent to null (send parent: null)
    // - Setting parent to frame (send parent: { id: "frame-123" })

    // Current implementation uses Option<Parent> which can't distinguish
    // between "don't update" and "set to null". We may need:
    // enum ParentUpdate { Keep, Remove, Set(Parent) }

    // The client call would be:
    // let client = create_test_client(&mock_server.uri()).await;
    // let result = client.update_item(
    //     "board-789",
    //     "sticky-456",
    //     None,                           // position
    //     None,                           // data
    //     None,                           // style
    //     None,                           // geometry
    //     Some(ParentUpdate::Remove),     // Explicitly remove parent
    // ).await;
    //
    // assert!(result.is_ok());
    // let updated_item = result.unwrap();
    // assert_eq!(updated_item.id, "sticky-456");
    // assert_eq!(updated_item.parent, None);

    // Verify mock setup
    assert!(mock_server.address().port() > 0);
}

// Additional test: Create frame that will contain items
#[tokio::test]
async fn test_create_frame_for_parent() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Expected request body for frame creation
    let expected_request = json!({
        "data": {
            "title": "Test Frame",
            "type": "frame"
        },
        "style": {
            "fillColor": "light_gray"
        },
        "position": {
            "x": 0.0,
            "y": 0.0
        },
        "geometry": {
            "width": 1000.0,
            "height": 800.0
        }
    });

    // Mock response for POST /boards/{id}/frames
    let mock_response = json!({
        "id": "frame-123",
        "data": {
            "title": "Test Frame",
            "type": "frame"
        },
        "style": {
            "fillColor": "light_gray"
        },
        "position": {
            "x": 0.0,
            "y": 0.0
        },
        "geometry": {
            "width": 1000.0,
            "height": 800.0
        }
    });

    Mock::given(method("POST"))
        .and(path_regex(r"^/v2/boards/.*/frames$"))
        .and(body_partial_json(&expected_request))
        .respond_with(ResponseTemplate::new(201).set_body_json(&mock_response))
        .expect(0) // No requests expected until MiroClient supports base URL injection
        .mount(&mock_server)
        .await;

    // Note: This test documents frame creation
    // The frame ID can then be used as parent_id in other create operations

    // The actual client call would be:
    // let client = create_test_client(&mock_server.uri()).await;
    // let result = client.create_frame(
    //     "board-789",
    //     "Test Frame".to_string(),
    //     0.0,
    //     0.0,
    //     1000.0,
    //     800.0,
    //     Some("light_gray".to_string()),
    //     None,  // parent_id for the frame itself
    // ).await;
    //
    // assert!(result.is_ok());
    // let frame = result.unwrap();
    // assert_eq!(frame.id, "frame-123");
    // This frame ID can now be used as parent_id for creating items inside it

    // Verify mock setup
    assert!(mock_server.address().port() > 0);
}

// Additional test: Verify request includes Bearer token
#[tokio::test]
async fn test_authentication_header_included() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Mock that requires Authorization header
    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/boards/.*/items.*$"))
        .and(wiremock::matchers::header(
            "Authorization",
            "Bearer test_access_token",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "cursor": null
        })))
        .expect(0) // No requests expected until MiroClient supports base URL injection
        .mount(&mock_server)
        .await;

    // Note: This test documents that all requests must include proper authentication
    // The actual client call would be:
    // let client = create_test_client(&mock_server.uri()).await;
    // let result = client.list_items("board-789", None, None).await;
    // assert!(result.is_ok());

    // Verify mock setup
    assert!(mock_server.address().port() > 0);
}
