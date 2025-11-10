/// Builder pattern implementations for complex Miro API operations
///
/// This module provides fluent builder APIs for methods with many parameters,
/// improving readability and making optional parameters explicit.
use crate::miro::client::{MiroClient, MiroError};
use crate::miro::types::{
    Caption, ConnectorResponse, ShapeResponse, StickyNoteResponse, TextResponse,
};

/// Builder for creating sticky notes with fluent API
///
/// # Example
/// ```no_run
/// # use miro_mcp_server::miro::client::MiroClient;
/// # use miro_mcp_server::miro::builders::StickyNoteBuilder;
/// # async fn example(client: &MiroClient) -> Result<(), Box<dyn std::error::Error>> {
/// let note = StickyNoteBuilder::new("board-id", "Hello World", 0.0, 100.0)
///     .color("light_yellow")
///     .parent_id("frame-123")
///     .build(client)
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct StickyNoteBuilder {
    board_id: String,
    content: String,
    x: f64,
    y: f64,
    color: String,
    parent_id: Option<String>,
}

impl StickyNoteBuilder {
    /// Create a new sticky note builder
    ///
    /// # Arguments
    /// * `board_id` - Board ID to create sticky note on
    /// * `content` - Content text for the sticky note
    /// * `x` - X coordinate (center of note)
    /// * `y` - Y coordinate (center of note)
    pub fn new(board_id: impl Into<String>, content: impl Into<String>, x: f64, y: f64) -> Self {
        Self {
            board_id: board_id.into(),
            content: content.into(),
            x,
            y,
            color: "light_yellow".to_string(), // Default color
            parent_id: None,
        }
    }

    /// Set the fill color of the sticky note
    ///
    /// # Valid Colors
    /// light_yellow, yellow, orange, light_green, green, dark_green,
    /// cyan, light_pink, pink, violet, red, light_blue, blue, dark_blue,
    /// gray, black
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    /// Set the parent frame or item ID
    pub fn parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Build and create the sticky note
    pub async fn build(self, client: &MiroClient) -> Result<StickyNoteResponse, MiroError> {
        client
            .create_sticky_note(
                &self.board_id,
                self.content,
                self.x,
                self.y,
                self.color,
                self.parent_id,
            )
            .await
    }
}

/// Builder for creating shapes with fluent API
///
/// # Example
/// ```no_run
/// # use miro_mcp_server::miro::client::MiroClient;
/// # use miro_mcp_server::miro::builders::ShapeBuilder;
/// # async fn example(client: &MiroClient) -> Result<(), Box<dyn std::error::Error>> {
/// let shape = ShapeBuilder::new("board-id", "rectangle", 0.0, 100.0, 200.0, 100.0)
///     .fill_color("light_blue")
///     .content("<p>Shape content</p>")
///     .parent_id("frame-123")
///     .build(client)
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct ShapeBuilder {
    board_id: String,
    shape_type: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill_color: String,
    content: Option<String>,
    parent_id: Option<String>,
}

impl ShapeBuilder {
    /// Create a new shape builder
    ///
    /// # Arguments
    /// * `board_id` - Board ID to create shape on
    /// * `shape_type` - Type of shape (rectangle, circle, triangle, etc.)
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// * `width` - Width in pixels
    /// * `height` - Height in pixels
    pub fn new(
        board_id: impl Into<String>,
        shape_type: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            board_id: board_id.into(),
            shape_type: shape_type.into(),
            x,
            y,
            width,
            height,
            fill_color: "light_blue".to_string(), // Default color
            content: None,
            parent_id: None,
        }
    }

    /// Set the fill color of the shape
    pub fn fill_color(mut self, color: impl Into<String>) -> Self {
        self.fill_color = color.into();
        self
    }

    /// Set the content (HTML) for the shape
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the parent frame or item ID
    pub fn parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Build and create the shape
    pub async fn build(self, client: &MiroClient) -> Result<ShapeResponse, MiroError> {
        client
            .create_shape(
                &self.board_id,
                self.shape_type,
                self.fill_color,
                self.x,
                self.y,
                self.width,
                self.height,
                self.content,
                self.parent_id,
            )
            .await
    }
}

/// Builder for creating text items with fluent API
///
/// # Example
/// ```no_run
/// # use miro_mcp_server::miro::client::MiroClient;
/// # use miro_mcp_server::miro::builders::TextBuilder;
/// # async fn example(client: &MiroClient) -> Result<(), Box<dyn std::error::Error>> {
/// let text = TextBuilder::new("board-id", "Hello World", 0.0, 100.0, 300.0)
///     .parent_id("frame-123")
///     .build(client)
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct TextBuilder {
    board_id: String,
    content: String,
    x: f64,
    y: f64,
    width: f64,
    parent_id: Option<String>,
}

impl TextBuilder {
    /// Create a new text builder
    ///
    /// # Arguments
    /// * `board_id` - Board ID to create text on
    /// * `content` - Text content
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// * `width` - Width in pixels
    pub fn new(
        board_id: impl Into<String>,
        content: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
    ) -> Self {
        Self {
            board_id: board_id.into(),
            content: content.into(),
            x,
            y,
            width,
            parent_id: None,
        }
    }

    /// Set the parent frame or item ID
    pub fn parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Build and create the text item
    pub async fn build(self, client: &MiroClient) -> Result<TextResponse, MiroError> {
        client
            .create_text(
                &self.board_id,
                self.content,
                self.x,
                self.y,
                self.width,
                self.parent_id,
            )
            .await
    }
}

/// Builder for creating connectors with fluent API
///
/// # Example
/// ```no_run
/// # use miro_mcp_server::miro::client::MiroClient;
/// # use miro_mcp_server::miro::builders::ConnectorBuilder;
/// # async fn example(client: &MiroClient) -> Result<(), Box<dyn std::error::Error>> {
/// let connector = ConnectorBuilder::new("board-id", "item-1", "item-2")
///     .stroke_color("blue")
///     .stroke_width(2.5)
///     .start_cap("none")
///     .end_cap("arrow")
///     .caption("depends on", Some(0.5))
///     .build(client)
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct ConnectorBuilder {
    board_id: String,
    start_item_id: String,
    end_item_id: String,
    stroke_color: Option<String>,
    stroke_width: Option<f64>,
    start_cap: Option<String>,
    end_cap: Option<String>,
    captions: Vec<Caption>,
}

impl ConnectorBuilder {
    /// Create a new connector builder
    ///
    /// # Arguments
    /// * `board_id` - Board ID to create connector on
    /// * `start_item_id` - ID of starting item
    /// * `end_item_id` - ID of ending item
    pub fn new(
        board_id: impl Into<String>,
        start_item_id: impl Into<String>,
        end_item_id: impl Into<String>,
    ) -> Self {
        Self {
            board_id: board_id.into(),
            start_item_id: start_item_id.into(),
            end_item_id: end_item_id.into(),
            stroke_color: None,
            stroke_width: None,
            start_cap: None,
            end_cap: None,
            captions: Vec::new(),
        }
    }

    /// Set the stroke color
    pub fn stroke_color(mut self, color: impl Into<String>) -> Self {
        self.stroke_color = Some(color.into());
        self
    }

    /// Set the stroke width in pixels
    pub fn stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = Some(width);
        self
    }

    /// Set the start cap style (none, arrow, etc.)
    pub fn start_cap(mut self, cap: impl Into<String>) -> Self {
        self.start_cap = Some(cap.into());
        self
    }

    /// Set the end cap style (none, arrow, etc.)
    pub fn end_cap(mut self, cap: impl Into<String>) -> Self {
        self.end_cap = Some(cap.into());
        self
    }

    /// Add a caption to the connector
    ///
    /// # Arguments
    /// * `content` - Caption text
    /// * `position` - Optional position (0.0 = start, 1.0 = end, 0.5 = middle)
    pub fn caption(mut self, content: impl Into<String>, position: Option<f64>) -> Self {
        self.captions.push(Caption {
            content: content.into(),
            position,
        });
        self
    }

    /// Build and create the connector
    pub async fn build(self, client: &MiroClient) -> Result<ConnectorResponse, MiroError> {
        let captions = if self.captions.is_empty() {
            None
        } else {
            Some(self.captions)
        };

        client
            .create_connector(
                &self.board_id,
                self.start_item_id,
                self.end_item_id,
                self.stroke_color,
                self.stroke_width,
                self.start_cap,
                self.end_cap,
                captions,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sticky_note_builder_construction() {
        let builder = StickyNoteBuilder::new("board-123", "Test Content", 100.0, 200.0)
            .color("yellow")
            .parent_id("frame-456");

        assert_eq!(builder.board_id, "board-123");
        assert_eq!(builder.content, "Test Content");
        assert_eq!(builder.x, 100.0);
        assert_eq!(builder.y, 200.0);
        assert_eq!(builder.color, "yellow");
        assert_eq!(builder.parent_id, Some("frame-456".to_string()));
    }

    #[test]
    fn test_sticky_note_builder_defaults() {
        let builder = StickyNoteBuilder::new("board-123", "Test", 0.0, 0.0);

        assert_eq!(builder.color, "light_yellow");
        assert_eq!(builder.parent_id, None);
    }

    #[test]
    fn test_shape_builder_construction() {
        let builder = ShapeBuilder::new("board-123", "rectangle", 0.0, 0.0, 200.0, 100.0)
            .fill_color("red")
            .content("<p>Text</p>")
            .parent_id("frame-456");

        assert_eq!(builder.board_id, "board-123");
        assert_eq!(builder.shape_type, "rectangle");
        assert_eq!(builder.fill_color, "red");
        assert_eq!(builder.content, Some("<p>Text</p>".to_string()));
        assert_eq!(builder.parent_id, Some("frame-456".to_string()));
    }

    #[test]
    fn test_shape_builder_defaults() {
        let builder = ShapeBuilder::new("board-123", "circle", 0.0, 0.0, 100.0, 100.0);

        assert_eq!(builder.fill_color, "light_blue");
        assert_eq!(builder.content, None);
        assert_eq!(builder.parent_id, None);
    }

    #[test]
    fn test_text_builder_construction() {
        let builder =
            TextBuilder::new("board-123", "Test Text", 0.0, 0.0, 300.0).parent_id("frame-456");

        assert_eq!(builder.board_id, "board-123");
        assert_eq!(builder.content, "Test Text");
        assert_eq!(builder.width, 300.0);
        assert_eq!(builder.parent_id, Some("frame-456".to_string()));
    }

    #[test]
    fn test_connector_builder_construction() {
        let builder = ConnectorBuilder::new("board-123", "item-1", "item-2")
            .stroke_color("blue")
            .stroke_width(2.5)
            .start_cap("none")
            .end_cap("arrow")
            .caption("depends on", Some(0.5));

        assert_eq!(builder.board_id, "board-123");
        assert_eq!(builder.start_item_id, "item-1");
        assert_eq!(builder.end_item_id, "item-2");
        assert_eq!(builder.stroke_color, Some("blue".to_string()));
        assert_eq!(builder.stroke_width, Some(2.5));
        assert_eq!(builder.start_cap, Some("none".to_string()));
        assert_eq!(builder.end_cap, Some("arrow".to_string()));
        assert_eq!(builder.captions.len(), 1);
        assert_eq!(builder.captions[0].content, "depends on");
        assert_eq!(builder.captions[0].position, Some(0.5));
    }

    #[test]
    fn test_connector_builder_multiple_captions() {
        let builder = ConnectorBuilder::new("board-123", "item-1", "item-2")
            .caption("first", Some(0.3))
            .caption("second", Some(0.7));

        assert_eq!(builder.captions.len(), 2);
        assert_eq!(builder.captions[0].content, "first");
        assert_eq!(builder.captions[1].content, "second");
    }

    #[test]
    fn test_connector_builder_defaults() {
        let builder = ConnectorBuilder::new("board-123", "item-1", "item-2");

        assert_eq!(builder.stroke_color, None);
        assert_eq!(builder.stroke_width, None);
        assert_eq!(builder.start_cap, None);
        assert_eq!(builder.end_cap, None);
        assert_eq!(builder.captions.len(), 0);
    }
}
