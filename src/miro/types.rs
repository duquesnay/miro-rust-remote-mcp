use serde::{Deserialize, Serialize};

/// Represents a Miro board
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Board {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: String,
}

/// API response for list boards endpoint
#[derive(Debug, Deserialize)]
pub struct BoardsResponse {
    pub data: Vec<Board>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Request body for creating a board
#[derive(Debug, Serialize)]
pub struct CreateBoardRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Response body for single board creation
#[derive(Debug, Deserialize)]
pub struct CreateBoardResponse {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: String,
}

/// Position for visual elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

/// Geometry dimensions for visual elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geometry {
    pub width: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
}

/// Sticky note data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyNoteData {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
}

/// Sticky note style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyNoteStyle {
    #[serde(rename = "fillColor")]
    pub fill_color: String,
}

/// Request body for creating a sticky note
#[derive(Debug, Clone, Serialize)]
pub struct CreateStickyNoteRequest {
    pub data: StickyNoteData,
    pub style: StickyNoteStyle,
    pub position: Position,
    pub geometry: Geometry,
}

/// Response for sticky note creation
#[derive(Debug, Clone, Deserialize)]
pub struct StickyNoteResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<StickyNoteData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<StickyNoteStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
}

/// Shape data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub shape: String,
}

/// Shape style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeStyle {
    #[serde(rename = "fillColor")]
    pub fill_color: String,
    #[serde(rename = "borderColor", skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(rename = "borderWidth", skip_serializing_if = "Option::is_none")]
    pub border_width: Option<String>,
}

/// Request body for creating a shape
#[derive(Debug, Clone, Serialize)]
pub struct CreateShapeRequest {
    pub data: ShapeData,
    pub style: ShapeStyle,
    pub position: Position,
    pub geometry: Geometry,
}

/// Response for shape creation
#[derive(Debug, Clone, Deserialize)]
pub struct ShapeResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ShapeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ShapeStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
}

/// Text data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextData {
    pub content: String,
}

/// Request body for creating text
#[derive(Debug, Clone, Serialize)]
pub struct CreateTextRequest {
    pub data: TextData,
    pub position: Position,
    pub geometry: Geometry,
}

/// Response for text creation
#[derive(Debug, Clone, Deserialize)]
pub struct TextResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<TextData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
}

/// Frame data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameData {
    pub title: String,
    #[serde(rename = "type")]
    pub frame_type: String,
}

/// Frame style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameStyle {
    #[serde(rename = "fillColor")]
    pub fill_color: String,
}

/// Request body for creating a frame
#[derive(Debug, Clone, Serialize)]
pub struct CreateFrameRequest {
    pub data: FrameData,
    pub style: FrameStyle,
    pub position: Position,
    pub geometry: Geometry,
}

/// Response for frame creation
#[derive(Debug, Clone, Deserialize)]
pub struct FrameResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<FrameData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<FrameStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_serialization() {
        let board = Board {
            id: "board-123".to_string(),
            name: "Test Board".to_string(),
            description: Some("A test board".to_string()),
            created_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&board).unwrap();
        assert!(json.contains("board-123"));
        assert!(json.contains("Test Board"));
    }

    #[test]
    fn test_board_deserialization() {
        let json = r#"{
            "id": "board-456",
            "name": "Another Board",
            "description": "Test description",
            "created_at": "2025-01-02T00:00:00Z"
        }"#;

        let board: Board = serde_json::from_str(json).unwrap();
        assert_eq!(board.id, "board-456");
        assert_eq!(board.name, "Another Board");
        assert_eq!(board.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_create_board_request() {
        let request = CreateBoardRequest {
            name: "New Board".to_string(),
            description: Some("New board description".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("New Board"));
        assert!(json.contains("New board description"));
    }

    #[test]
    fn test_create_board_request_without_description() {
        let request = CreateBoardRequest {
            name: "Board Without Desc".to_string(),
            description: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("description")); // Should be skipped when None
        assert!(json.contains("Board Without Desc"));
    }

    #[test]
    fn test_sticky_note_request_serialization() {
        let request = CreateStickyNoteRequest {
            data: StickyNoteData {
                content: "<p>Test note</p>".to_string(),
                shape: Some("square".to_string()),
            },
            style: StickyNoteStyle {
                fill_color: "light_yellow".to_string(),
            },
            position: Position {
                x: 100.0,
                y: 200.0,
                origin: Some("center".to_string()),
            },
            geometry: Geometry {
                width: 200.0,
                height: None,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test note"));
        assert!(json.contains("light_yellow"));
        assert!(json.contains("\"x\":100"));
        assert!(json.contains("\"y\":200"));
    }

    #[test]
    fn test_shape_request_serialization() {
        let request = CreateShapeRequest {
            data: ShapeData {
                content: Some("<p>Shape text</p>".to_string()),
                shape: "rectangle".to_string(),
            },
            style: ShapeStyle {
                fill_color: "light_blue".to_string(),
                border_color: Some("blue".to_string()),
                border_width: Some("2".to_string()),
            },
            position: Position {
                x: 0.0,
                y: 0.0,
                origin: None,
            },
            geometry: Geometry {
                width: 300.0,
                height: Some(150.0),
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("rectangle"));
        assert!(json.contains("light_blue"));
        assert!(json.contains("blue"));
        assert!(json.contains("\"width\":300"));
    }

    #[test]
    fn test_text_request_serialization() {
        let request = CreateTextRequest {
            data: TextData {
                content: "Plain text content".to_string(),
            },
            position: Position {
                x: 50.0,
                y: 75.0,
                origin: None,
            },
            geometry: Geometry {
                width: 200.0,
                height: None,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Plain text content"));
        assert!(json.contains("\"x\":50"));
    }

    #[test]
    fn test_frame_request_serialization() {
        let request = CreateFrameRequest {
            data: FrameData {
                title: "Frame Title".to_string(),
                frame_type: "frame".to_string(),
            },
            style: FrameStyle {
                fill_color: "light_gray".to_string(),
            },
            position: Position {
                x: 0.0,
                y: 0.0,
                origin: None,
            },
            geometry: Geometry {
                width: 1000.0,
                height: Some(800.0),
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Frame Title"));
        assert!(json.contains("light_gray"));
        assert!(json.contains("\"width\":1000"));
        assert!(json.contains("\"height\":800"));
    }

    #[test]
    fn test_sticky_note_response_deserialization() {
        let json = r#"{
            "id": "note-123",
            "data": {
                "content": "<p>Test</p>",
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
            }
        }"#;

        let response: StickyNoteResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "note-123");
        assert!(response.data.is_some());
    }
}
