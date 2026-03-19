//! Multi-Question Dialog Component
//!
//! A modal dialog for multi-question prompts with various selection modes.
//! Supports single-select, multi-select, and custom answer options.
//!
//! # Features
//!
//! - Multi-question support with tab navigation
//! - Single-question mode with auto-submit
//! - Multi-select mode with checkboxes `[x]` / `[ ]`
//! - Custom answer option support
//! - Quick selection via number keys (1-9)
//! - Keyboard navigation (arrow keys, Tab)
//!
//! # Examples
//!
//! ```rust,ignore
//! let options = vec![
//!     QuestionOption::new("Option 1"),
//!     QuestionOption::new("Option 2").with_description("Description"),
//! ];
//!
//! let question = Question::new("Header", "What do you want?", options)
//!     .with_multiple(true)
//!     .with_custom(true);
//!
//! let request = QuestionRequest::new("req-1", vec![question]);
//! let mut dialog = MultiQuestionDialog::new(request);
//! dialog.show();
//! ```

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// =============================================================================
// Data Models
// =============================================================================

/// Question option for selection
///
/// Represents a single selectable option within a question.
/// Options can have an optional description shown below the label.
///
/// # Examples
///
/// ```
/// let option = QuestionOption::new("Save");
/// assert_eq!(option.label, "Save");
///
/// let option = QuestionOption::new("Open").with_description("Open a file");
/// assert_eq!(option.description, Some("Open a file".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display label for the option
    pub label: String,
    /// Optional description shown below the label
    pub description: Option<String>,
}

impl QuestionOption {
    /// Create a new QuestionOption with the given label
    ///
    /// # Arguments
    ///
    /// * `label` - The display label for the option
    ///
    /// # Examples
    ///
    /// ```
    /// let option = QuestionOption::new("Option 1");
    /// assert_eq!(option.label, "Option 1");
    /// assert_eq!(option.description, None);
    /// ```
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }

    /// Builder: Set the description
    ///
    /// # Arguments
    ///
    /// * `desc` - The description text
    ///
    /// # Examples
    ///
    /// ```
    /// let option = QuestionOption::new("Save")
    ///     .with_description("Save the current file");
    /// assert_eq!(option.description, Some("Save the current file".to_string()));
    /// ```
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A single question
///
/// Represents a question with multiple options and configuration
/// for multi-select and custom answer support.
///
/// # Examples
///
/// ```
/// let options = vec![
///     QuestionOption::new("Option A"),
///     QuestionOption::new("Option B"),
/// ];
/// let question = Question::new("Header", "Which one?", options);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    /// Header text shown in the tab (for multi-question mode)
    pub header: String,
    /// The actual question text
    pub question: String,
    /// Available options
    pub options: Vec<QuestionOption>,
    /// Whether multiple selections are allowed
    pub multiple: bool,
    /// Whether custom answer input is allowed
    pub custom: bool,
}

impl Question {
    /// Create a new Question
    ///
    /// # Arguments
    ///
    /// * `header` - Header text for the tab
    /// * `question` - The question text
    /// * `options` - Vector of QuestionOption
    ///
    /// # Examples
    ///
    /// ```
    /// let options = vec![QuestionOption::new("Yes"), QuestionOption::new("No")];
    /// let question = Question::new("Confirm", "Are you sure?", options);
    /// assert_eq!(question.header, "Confirm");
    /// assert_eq!(question.question, "Are you sure?");
    /// assert!(!question.multiple);
    /// assert!(!question.custom);
    /// ```
    pub fn new(
        header: impl Into<String>,
        question: impl Into<String>,
        options: Vec<QuestionOption>,
    ) -> Self {
        Self {
            header: header.into(),
            question: question.into(),
            options,
            multiple: false,
            custom: false,
        }
    }

    /// Builder: Enable multi-select mode
    ///
    /// # Examples
    ///
    /// ```
    /// let options = vec![QuestionOption::new("A"), QuestionOption::new("B")];
    /// let question = Question::new("Select", "Choose all", options)
    ///     .with_multiple(true);
    /// assert!(question.multiple);
    /// ```
    pub fn with_multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Builder: Enable custom answer
    ///
    /// # Examples
    ///
    /// ```
    /// let options = vec![QuestionOption::new("A")];
    /// let question = Question::new("Select", "Choose", options)
    ///     .with_custom(true);
    /// assert!(question.custom);
    /// ```
    pub fn with_custom(mut self, custom: bool) -> Self {
        self.custom = custom;
        self
    }
}

/// Question request from agent
///
/// Contains a request ID and a list of questions to present to the user.
///
/// # Examples
///
/// ```
/// let questions = vec![
///     Question::new("Q1", "First question?", vec![QuestionOption::new("Yes")]),
/// ];
/// let request = QuestionRequest::new("req-1", questions);
/// ```
#[derive(Debug, Clone)]
pub struct QuestionRequest {
    /// Unique request identifier
    pub id: String,
    /// List of questions
    pub questions: Vec<Question>,
}

impl QuestionRequest {
    /// Create a new QuestionRequest
    ///
    /// # Arguments
    ///
    /// * `id` - Unique request ID
    /// * `questions` - Vector of Question
    ///
    /// # Examples
    ///
    /// ```
    /// let request = QuestionRequest::new("req-1", vec![]);
    /// assert_eq!(request.id, "req-1");
    /// assert!(request.questions.is_empty());
    /// ```
    pub fn new(id: impl Into<String>, questions: Vec<Question>) -> Self {
        Self {
            id: id.into(),
            questions,
        }
    }
}

/// Answer to submit
///
/// One answer per question. For multi-select, contains multiple values.
pub type QuestionAnswer = Vec<String>;

// =============================================================================
// QuestionDialog Component
// =============================================================================

/// Dialog for multi-question prompts
///
/// A modal dialog that handles multi-question prompts with various
/// selection modes including single-select, multi-select, and custom answers.
///
/// For single-question mode, tabs are hidden and auto-submit on selection.
/// For multi-question mode, tabs are shown with a confirm tab at the end.
///
/// # Examples
///
/// ```
/// let options = vec![QuestionOption::new("Yes"), QuestionOption::new("No")];
/// let question = Question::new("Confirm", "Are you sure?", options);
/// let request = QuestionRequest::new("req-1", vec![question]);
/// let dialog = MultiMultiQuestionDialog::new(request);
///
/// assert!(!dialog.is_visible());
/// assert!(dialog.is_single_question());
/// ```
pub struct MultiQuestionDialog {
    /// Unique component identifier
    id: ComponentId,
    /// The question request
    request: QuestionRequest,
    /// Current question index (or questions.len() for confirm tab)
    current_tab: usize,
    /// Selected option in current question
    selected_option: usize,
    /// Answers for each question (multiple strings for multi-select)
    answers: Vec<Vec<String>>,
    /// Custom text inputs
    custom_inputs: Vec<String>,
    /// Whether editing custom answer
    editing_custom: bool,
    /// Whether the dialog is visible
    visible: bool,
    /// Whether the dialog has focus
    focused: bool,
}

impl MultiQuestionDialog {
    /// Create a new QuestionDialog
    ///
    /// # Arguments
    ///
    /// * `request` - The QuestionRequest containing questions
    ///
    /// # Examples
    ///
    /// ```
    /// let options = vec![QuestionOption::new("A")];
    /// let question = Question::new("Q", "Question?", options);
    /// let request = QuestionRequest::new("req-1", vec![question]);
    /// let dialog = MultiQuestionDialog::new(request);
    ///
    /// assert!(!dialog.is_visible());
    /// assert_eq!(dialog.tab_count(), 1);
    /// ```
    pub fn new(request: QuestionRequest) -> Self {
        let num_questions = request.questions.len();
        Self {
            id: ComponentId::new(),
            request,
            current_tab: 0,
            selected_option: 0,
            answers: vec![Vec::new(); num_questions],
            custom_inputs: vec![String::new(); num_questions],
            editing_custom: false,
            visible: false,
            focused: false,
        }
    }

    /// Check if this is a single-question dialog
    ///
    /// Single-question mode has no tabs and auto-submits on selection.
    ///
    /// # Returns
    ///
    /// `true` if there's exactly one question and it's not multi-select.
    ///
    /// # Examples
    ///
    /// ```
    /// let options = vec![QuestionOption::new("A")];
    ///
    /// // Single question, not multi-select
    /// let q1 = Question::new("Q1", "Question?", options.clone());
    /// let r1 = QuestionRequest::new("req-1", vec![q1]);
    /// let d1 = MultiQuestionDialog::new(r1);
    /// assert!(d1.is_single_question());
    ///
    /// // Multi-select question
    /// let q2 = Question::new("Q2", "Question?", options.clone()).with_multiple(true);
    /// let r2 = QuestionRequest::new("req-2", vec![q2]);
    /// let d2 = MultiQuestionDialog::new(r2);
    /// assert!(!d2.is_single_question());
    ///
    /// // Multiple questions
    /// let r3 = QuestionRequest::new("req-3", vec![
    ///     Question::new("Q1", "First?", options.clone()),
    ///     Question::new("Q2", "Second?", options.clone()),
    /// ]);
    /// let d3 = MultiQuestionDialog::new(r3);
    /// assert!(!d3.is_single_question());
    /// ```
    pub fn is_single_question(&self) -> bool {
        self.request.questions.len() == 1
            && !self
                .request
                .questions
                .first()
                .map(|q| q.multiple)
                .unwrap_or(false)
    }

    /// Get the total number of tabs
    ///
    /// For single-question mode, returns 1.
    /// For multi-question mode, returns questions.len() + 1 (for confirm tab).
    ///
    /// # Examples
    ///
    /// ```
    /// let options = vec![QuestionOption::new("A")];
    ///
    /// // Single question
    /// let r1 = QuestionRequest::new("req-1", vec![
    ///     Question::new("Q1", "Question?", options.clone()),
    /// ]);
    /// let d1 = MultiQuestionDialog::new(r1);
    /// assert_eq!(d1.tab_count(), 1);
    ///
    /// // Multiple questions
    /// let r2 = QuestionRequest::new("req-2", vec![
    ///     Question::new("Q1", "First?", options.clone()),
    ///     Question::new("Q2", "Second?", options.clone()),
    /// ]);
    /// let d2 = MultiQuestionDialog::new(r2);
    /// assert_eq!(d2.tab_count(), 3); // 2 questions + confirm tab
    /// ```
    pub fn tab_count(&self) -> usize {
        if self.is_single_question() {
            1
        } else {
            self.request.questions.len() + 1 // questions + confirm tab
        }
    }

    /// Check if currently on the confirm tab
    ///
    /// # Returns
    ///
    /// `true` if on confirm tab (only applicable for multi-question mode).
    fn is_confirm_tab(&self) -> bool {
        !self.is_single_question() && self.current_tab >= self.request.questions.len()
    }

    /// Get the current question (if not on confirm tab)
    ///
    /// # Returns
    ///
    /// Some(&Question) if on a question tab, None if on confirm tab.
    fn current_question(&self) -> Option<&Question> {
        if self.is_confirm_tab() {
            None
        } else {
            self.request.questions.get(self.current_tab)
        }
    }

    /// Get the number of options in current question
    ///
    /// Includes custom option if enabled.
    fn option_count(&self) -> usize {
        match self.current_question() {
            Some(question) => {
                let base = question.options.len();
                if question.custom {
                    base + 1
                } else {
                    base
                }
            }
            None => 0,
        }
    }

    /// Check if currently selected option is the custom option
    fn is_custom_selected(&self) -> bool {
        let question = match self.current_question() {
            Some(q) => q,
            None => return false,
        };
        if !question.custom {
            return false;
        }
        self.selected_option == question.options.len()
    }

    /// Check if an option is selected (for multi-select mode)
    ///
    /// # Arguments
    ///
    /// * `index` - The option index to check
    ///
    /// # Returns
    ///
    /// `true` if the option is selected.
    fn is_option_selected(&self, index: usize) -> bool {
        if self.current_tab >= self.answers.len() {
            return false;
        }
        let question = match self.current_question() {
            Some(q) => q,
            None => return false,
        };
        if index >= question.options.len() {
            return false;
        }
        let label = &question.options[index].label;
        self.answers[self.current_tab].contains(label)
    }

    /// Check if custom input is selected
    fn is_custom_input_selected(&self) -> bool {
        if !self.is_custom_selected() {
            return false;
        }
        let custom = &self.custom_inputs[self.current_tab];
        !custom.is_empty() && self.answers[self.current_tab].contains(custom)
    }

    /// Toggle selection of an option (for multi-select)
    ///
    /// # Arguments
    ///
    /// * `index` - The option index to toggle
    fn toggle_option(&mut self, index: usize) {
        let question = match self.current_question() {
            Some(q) => q,
            None => return,
        };
        if index >= question.options.len() {
            return;
        }

        let label = question.options[index].label.clone();
        let answers = &mut self.answers[self.current_tab];

        if let Some(pos) = answers.iter().position(|a| a == &label) {
            answers.remove(pos);
        } else {
            answers.push(label);
        }
    }

    /// Select a single option (for single-select mode)
    ///
    /// # Arguments
    ///
    /// * `index` - The option index to select
    fn select_single(&mut self, index: usize) {
        let question = match self.current_question() {
            Some(q) => q,
            None => return,
        };
        if index >= question.options.len() {
            return;
        }

        let label = question.options[index].label.clone();
        self.answers[self.current_tab] = vec![label];
    }

    /// Select current option
    ///
    /// Handles both single-select and multi-select modes.
    fn select_current(&mut self) {
        if self.is_custom_selected() {
            // Custom option selected - enter edit mode
            self.editing_custom = true;
            return;
        }

        let question = match self.current_question() {
            Some(q) => q,
            None => return,
        };

        if question.multiple {
            self.toggle_option(self.selected_option);
        } else {
            self.select_single(self.selected_option);
            if self.is_single_question() {
                // Auto-submit for single-question mode
                self.visible = false;
            } else {
                // Move to next tab
                self.next_tab();
            }
        }
    }

    /// Navigate to the next tab
    fn next_tab(&mut self) {
        let count = self.tab_count();
        self.current_tab = (self.current_tab + 1) % count;
        self.selected_option = 0;
        self.editing_custom = false;
    }

    /// Navigate to the previous tab
    fn prev_tab(&mut self) {
        let count = self.tab_count();
        self.current_tab = (self.current_tab + count - 1) % count;
        self.selected_option = 0;
        self.editing_custom = false;
    }

    /// Select a specific tab
    ///
    /// # Arguments
    ///
    /// * `index` - The tab index to select
    fn select_tab(&mut self, index: usize) {
        let count = self.tab_count();
        if index < count {
            self.current_tab = index;
            self.selected_option = 0;
            self.editing_custom = false;
        }
    }

    /// Navigate to next option
    fn next_option(&mut self) {
        let count = self.option_count();
        if count > 0 {
            self.selected_option = (self.selected_option + 1) % count;
        }
    }

    /// Navigate to previous option
    fn prev_option(&mut self) {
        let count = self.option_count();
        if count > 0 {
            self.selected_option = (self.selected_option + count - 1) % count;
        }
    }

    /// Select option by index (for number keys)
    ///
    /// # Arguments
    ///
    /// * `index` - The 0-based option index
    fn select_option_index(&mut self, index: usize) {
        let count = self.option_count();
        if index < count {
            self.selected_option = index;
            self.select_current();
        }
    }

    /// Submit answers
    fn submit(&mut self) {
        self.visible = false;
        self.focused = false;
    }

    /// Reject/dismiss dialog
    fn reject(&mut self) {
        self.visible = false;
        self.focused = false;
        // Clear answers to indicate rejection
        self.answers = vec![Vec::new(); self.request.questions.len()];
    }

    /// Get the request ID
    ///
    /// # Returns
    ///
    /// The request ID as a string slice.
    pub fn request_id(&self) -> &str {
        &self.request.id
    }

    /// Get the answers
    ///
    /// # Returns
    ///
    /// A slice of QuestionAnswer (one per question).
    pub fn answers(&self) -> &[QuestionAnswer] {
        &self.answers
    }

    /// Check if the dialog is visible
    ///
    /// # Returns
    ///
    /// `true` if the dialog is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the dialog
    ///
    /// Makes the dialog visible and focused.
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
        self.current_tab = 0;
        self.selected_option = 0;
        self.editing_custom = false;
    }

    /// Hide the dialog
    ///
    /// Makes the dialog invisible and unfocused.
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
    }

    /// Check if the dialog is focused
    ///
    /// # Returns
    ///
    /// `true` if the dialog has focus.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Get the current tab index
    ///
    /// # Returns
    ///
    /// The current tab index.
    pub fn current_tab(&self) -> usize {
        self.current_tab
    }

    /// Get the selected option index
    ///
    /// # Returns
    ///
    /// The selected option index.
    pub fn selected_option(&self) -> usize {
        self.selected_option
    }

    /// Calculate the centered area for the dialog
    ///
    /// # Arguments
    ///
    /// * `area` - The total available area
    ///
    /// # Returns
    ///
    /// A Rect representing the centered dialog area.
    fn centered_area(&self, area: Rect) -> Rect {
        let width = area.width * 70 / 100; // 70% width
        let height = area.height * 70 / 100; // 70% height
        let x = area.x + (area.width - width) / 2;
        let y = area.y + (area.height - height) / 2;
        Rect::new(x, y, width, height)
    }
}

impl Component for MultiQuestionDialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let dialog_area = self.centered_area(area);

        // Clear the area for modal overlay
        frame.render_widget(Clear, dialog_area);

        // Dialog block with border
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        };

        let block = Block::default()
            .title(" Question ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Create layout: tabs (if multi-question), content, help
        let mut constraints = vec![Constraint::Min(1)]; // Content area
        if !self.is_single_question() {
            constraints.insert(0, Constraint::Length(2)); // Tabs row
        }
        constraints.push(Constraint::Length(1)); // Help text

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let mut layout_idx = 0;

        // Render tabs for multi-question mode
        if !self.is_single_question() {
            self.render_tabs(frame, layout[layout_idx]);
            layout_idx += 1;
        }

        // Render content (question or confirm)
        if self.is_confirm_tab() {
            self.render_confirm(frame, layout[layout_idx]);
        } else {
            self.render_question(frame, layout[layout_idx]);
        }
        layout_idx += 1;

        // Render help text
        self.render_help(frame, layout[layout_idx]);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::KeyCode;

        if !self.visible {
            return EventPropagation::Continue;
        }

        // Handle custom editing mode
        if self.editing_custom {
            match event.code {
                KeyCode::Esc => {
                    self.editing_custom = false;
                    return EventPropagation::Stop;
                }
                KeyCode::Enter => {
                    // Save custom input
                    let custom = self.custom_inputs[self.current_tab].trim().to_string();
                    if !custom.is_empty() {
                        let question = match self.current_question() {
                            Some(q) => q,
                            None => return EventPropagation::Stop,
                        };

                        if question.multiple {
                            // Toggle the custom value
                            let answers = &mut self.answers[self.current_tab];
                            if let Some(pos) = answers.iter().position(|a| a == &custom) {
                                answers.remove(pos);
                            } else {
                                answers.push(custom);
                            }
                        } else {
                            // Select as single answer
                            self.answers[self.current_tab] = vec![custom];
                            if self.is_single_question() {
                                self.visible = false;
                                return EventPropagation::Stop;
                            }
                            self.next_tab();
                        }
                    }
                    self.editing_custom = false;
                    return EventPropagation::Stop;
                }
                _ => {
                    // For now, just consume other keys in edit mode
                    // Full text input would require additional state
                    return EventPropagation::Stop;
                }
            }
        }

        // Handle confirm tab
        if self.is_confirm_tab() {
            match event.code {
                KeyCode::Esc => {
                    self.reject();
                    EventPropagation::Stop
                }
                KeyCode::Enter => {
                    self.submit();
                    EventPropagation::Stop
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.prev_tab();
                    EventPropagation::Stop
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.next_tab();
                    EventPropagation::Stop
                }
                KeyCode::Tab => {
                    self.next_tab();
                    EventPropagation::Stop
                }
                _ => EventPropagation::Stop,
            }
        } else {
            // Handle question tab
            let question = match self.current_question() {
                Some(q) => q,
                None => return EventPropagation::Continue,
            };

            let opts_len = question.options.len();
            let total_options = self.option_count();

            match event.code {
                // Navigation
                KeyCode::Left | KeyCode::Char('h') => {
                    self.prev_tab();
                    EventPropagation::Stop
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.next_tab();
                    EventPropagation::Stop
                }
                KeyCode::Tab => {
                    self.next_tab();
                    EventPropagation::Stop
                }
                KeyCode::BackTab => {
                    self.prev_tab();
                    EventPropagation::Stop
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.prev_option();
                    EventPropagation::Stop
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.next_option();
                    EventPropagation::Stop
                }

                // Number keys for quick selection
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let digit = c as u8 - b'0';
                    if digit >= 1 && digit <= 9 {
                        let index = (digit - 1) as usize;
                        // Limit to available options
                        let max_options = total_options.min(9);
                        if index < max_options {
                            self.select_option_index(index);
                        }
                    }
                    EventPropagation::Stop
                }

                // Selection
                KeyCode::Enter => {
                    self.select_current();
                    EventPropagation::Stop
                }

                // Cancellation
                KeyCode::Esc => {
                    self.reject();
                    EventPropagation::Stop
                }

                // Consume other keys
                _ => EventPropagation::Stop,
            }
        }
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

// =============================================================================
// Rendering Helpers
// =============================================================================

impl MultiQuestionDialog {
    /// Render tabs for multi-question mode
    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let questions = &self.request.questions;
        let num_questions = questions.len();

        // Calculate tab widths
        let tab_count = num_questions + 1; // +1 for confirm tab
        let tab_width = area.width / tab_count as u16;

        for (i, question) in questions.iter().enumerate() {
            let is_active = i == self.current_tab;
            let is_answered = !self.answers[i].is_empty();

            let tab_x = area.x + (i as u16 * tab_width);
            let tab_area = Rect::new(tab_x, area.y, tab_width, 1);

            let (bg, fg) = if is_active {
                (Color::Cyan, Color::Black)
            } else if is_answered {
                (Color::DarkGray, Color::White)
            } else {
                (Color::Black, Color::DarkGray)
            };

            let header_text = format!(" {} ", question.header.as_str());
            let span = Span::styled(header_text, Style::default().bg(bg).fg(fg));
            let line = Line::from(span);
            frame.render_widget(Paragraph::new(line), tab_area);
        }

        // Render confirm tab
        let confirm_idx = num_questions;
        let is_confirm = self.current_tab == confirm_idx;
        let confirm_x = area.x + (confirm_idx as u16 * tab_width);
        let confirm_area = Rect::new(confirm_x, area.y, tab_width, 1);

        let (bg, fg) = if is_confirm {
            (Color::Cyan, Color::Black)
        } else {
            (Color::Black, Color::DarkGray)
        };

        let confirm_text = Span::styled(" Confirm ", Style::default().bg(bg).fg(fg));
        let confirm_line = Line::from(confirm_text);
        frame.render_widget(Paragraph::new(confirm_line), confirm_area);
    }

    /// Render the question content
    fn render_question(&self, frame: &mut Frame, area: Rect) {
        let question = match self.current_question() {
            Some(q) => q,
            None => return,
        };

        // Create layout: question text, options, custom (if enabled)
        let mut constraints = vec![Constraint::Length(1)]; // Question text
        constraints.push(Constraint::Min(1)); // Options list
        if question.custom {
            constraints.push(Constraint::Length(1)); // Custom option
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Render question text
        let question_suffix = if question.multiple {
            " (select all that apply)"
        } else {
            ""
        };
        let question_text = format!("{}{}", question.question, question_suffix);
        let question_line = Line::from(Span::styled(
            question_text,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(question_line), layout[0]);

        // Render options
        let options_area = layout[1];
        self.render_options(frame, options_area, question);
    }

    /// Render the options list
    fn render_options(&self, frame: &mut Frame, area: Rect, question: &Question) {
        for (i, option) in question.options.iter().enumerate() {
            if i >= area.height as usize {
                break;
            }

            let is_selected = self.selected_option == i;
            let is_picked = self.is_option_selected(i);

            let row_y = area.y + i as u16;
            let row_area = Rect::new(area.x, row_y, area.width, 1);

            // Build the option line
            let mut spans = vec![];

            // Number indicator
            let num_style = if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(format!("{}.", i + 1), num_style));
            spans.push(Span::raw(" "));

            // Checkbox for multi-select, checkmark for single-select
            if question.multiple {
                let checkbox = if is_picked { "[x]" } else { "[ ]" };
                let checkbox_style = if is_picked {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(checkbox, checkbox_style));
                spans.push(Span::raw(" "));
            } else if is_picked {
                spans.push(Span::styled("✓", Style::default().fg(Color::Green)));
                spans.push(Span::raw(" "));
            }

            // Option label
            let label_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_picked {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(&option.label, label_style));

            let line = Line::from(spans);
            frame.render_widget(Paragraph::new(line), row_area);

            // Render description if present (on next line)
            if let Some(desc) = &option.description {
                if i + 1 < area.height as usize {
                    let desc_y = row_y + 1;
                    let desc_area = Rect::new(area.x + 3, desc_y, area.width.saturating_sub(3), 1);
                    let desc_line = Line::from(Span::styled(
                        desc.as_str(),
                        Style::default().fg(Color::DarkGray),
                    ));
                    frame.render_widget(Paragraph::new(desc_line), desc_area);
                }
            }
        }

        // Render custom option if enabled
        if question.custom {
            let custom_idx = question.options.len();
            let is_custom = self.selected_option == custom_idx;
            let is_custom_picked = self.is_custom_input_selected();

            let custom_y = area.y + custom_idx as u16 * 2; // Account for descriptions
            if custom_y < area.y + area.height {
                let custom_area = Rect::new(area.x, custom_y, area.width, 1);

                let mut spans = vec![];

                // Number
                let num_style = if is_custom {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(format!("{}.", custom_idx + 1), num_style));
                spans.push(Span::raw(" "));

                // Checkbox or checkmark
                if question.multiple {
                    let checkbox = if is_custom_picked { "[x]" } else { "[ ]" };
                    let checkbox_style = if is_custom_picked {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    spans.push(Span::styled(checkbox, checkbox_style));
                    spans.push(Span::raw(" "));
                } else if is_custom_picked {
                    spans.push(Span::styled("✓", Style::default().fg(Color::Green)));
                    spans.push(Span::raw(" "));
                }

                // Label
                let label_style = if is_custom {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_custom_picked {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                spans.push(Span::styled("Type your own answer", label_style));

                let line = Line::from(spans);
                frame.render_widget(Paragraph::new(line), custom_area);

                // Show custom input if editing or has value
                if self.editing_custom || !self.custom_inputs[self.current_tab].is_empty() {
                    let input_y = custom_y + 1;
                    if input_y < area.y + area.height {
                        let input_area =
                            Rect::new(area.x + 3, input_y, area.width.saturating_sub(3), 1);
                        let input_text = if self.editing_custom {
                            format!("{}_", self.custom_inputs[self.current_tab])
                        // Show cursor
                        } else {
                            self.custom_inputs[self.current_tab].clone()
                        };
                        let input_line =
                            Line::from(Span::styled(input_text, Style::default().fg(Color::White)));
                        frame.render_widget(Paragraph::new(input_line), input_area);
                    }
                }
            }
        }
    }

    /// Render the confirm tab content
    fn render_confirm(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![];

        // Title
        lines.push(Line::from(Span::styled(
            "Review",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Show each question and its answer
        for (i, question) in self.request.questions.iter().enumerate() {
            let answer = if self.answers[i].is_empty() {
                Span::styled("(not answered)", Style::default().fg(Color::Red))
            } else {
                Span::styled(
                    self.answers[i].join(", "),
                    Style::default().fg(Color::White),
                )
            };

            let mut line_spans = vec![Span::styled(
                format!("{}: ", question.header),
                Style::default().fg(Color::DarkGray),
            )];
            line_spans.push(answer);
            lines.push(Line::from(line_spans));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    /// Render help text
    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![];

        if !self.is_single_question() {
            // Tab navigation
            spans.push(Span::styled(
                "←→",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" tab "));
        }

        if self.is_confirm_tab() {
            spans.push(Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" submit "));
        } else {
            spans.push(Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" select "));

            // Get current question to show correct action
            if let Some(question) = self.current_question() {
                let action = if question.multiple {
                    "toggle"
                } else {
                    "confirm"
                };
                spans.push(Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(format!(" {} ", action)));
            }
        }

        spans.push(Span::styled(
            "Esc",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" dismiss"));

        let line = Line::from(spans);
        frame.render_widget(
            Paragraph::new(line).alignment(ratatui::layout::Alignment::Center),
            area,
        );
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Data Model Tests
    // =============================================================================

    #[test]
    fn test_question_option_new() {
        let option = QuestionOption::new("Test");
        assert_eq!(option.label, "Test");
        assert_eq!(option.description, None);
    }

    #[test]
    fn test_question_option_with_description() {
        let option = QuestionOption::new("Test").with_description("A description");
        assert_eq!(option.label, "Test");
        assert_eq!(option.description, Some("A description".to_string()));
    }

    #[test]
    fn test_question_new() {
        let options = vec![QuestionOption::new("Yes"), QuestionOption::new("No")];
        let question = Question::new("Confirm", "Are you sure?", options);
        assert_eq!(question.header, "Confirm");
        assert_eq!(question.question, "Are you sure?");
        assert_eq!(question.options.len(), 2);
        assert!(!question.multiple);
        assert!(!question.custom);
    }

    #[test]
    fn test_question_with_multiple() {
        let question = Question::new("Select", "Choose", vec![]).with_multiple(true);
        assert!(question.multiple);
    }

    #[test]
    fn test_question_with_custom() {
        let question = Question::new("Select", "Choose", vec![]).with_custom(true);
        assert!(question.custom);
    }

    #[test]
    fn test_question_request_new() {
        let request = QuestionRequest::new("req-1", vec![]);
        assert_eq!(request.id, "req-1");
        assert!(request.questions.is_empty());
    }

    // =============================================================================
    // QuestionDialog Tests
    // =============================================================================

    fn create_test_options() -> Vec<QuestionOption> {
        vec![
            QuestionOption::new("Option 1").with_description("First option"),
            QuestionOption::new("Option 2"),
            QuestionOption::new("Option 3"),
        ]
    }

    #[test]
    fn test_question_dialog_new() {
        let options = create_test_options();
        let question = Question::new("Q1", "Test question?", options);
        let request = QuestionRequest::new("req-1", vec![question]);
        let dialog = MultiQuestionDialog::new(request);

        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
        assert_eq!(dialog.current_tab(), 0);
        assert_eq!(dialog.selected_option(), 0);
        assert!(dialog.is_single_question());
        assert_eq!(dialog.tab_count(), 1);
        assert_eq!(dialog.request_id(), "req-1");
    }

    #[test]
    fn test_question_dialog_show_hide() {
        let options = create_test_options();
        let question = Question::new("Q1", "Test?", options);
        let request = QuestionRequest::new("req-1", vec![question]);
        let mut dialog = MultiQuestionDialog::new(request);

        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());

        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_question_dialog_single_question_detection() {
        let options = create_test_options();

        // Single question, not multi-select
        let q1 = Question::new("Q1", "Test?", options.clone());
        let r1 = QuestionRequest::new("req-1", vec![q1]);
        let d1 = MultiQuestionDialog::new(r1);
        assert!(d1.is_single_question());

        // Multi-select question
        let q2 = Question::new("Q2", "Test?", options.clone()).with_multiple(true);
        let r2 = QuestionRequest::new("req-2", vec![q2]);
        let d2 = MultiQuestionDialog::new(r2);
        assert!(!d2.is_single_question());

        // Multiple questions
        let r3 = QuestionRequest::new(
            "req-3",
            vec![
                Question::new("Q1", "First?", options.clone()),
                Question::new("Q2", "Second?", options.clone()),
            ],
        );
        let d3 = MultiQuestionDialog::new(r3);
        assert!(!d3.is_single_question());
    }

    #[test]
    fn test_question_dialog_tab_count() {
        let options = create_test_options();

        // Single question
        let r1 = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options.clone())]);
        let d1 = MultiQuestionDialog::new(r1);
        assert_eq!(d1.tab_count(), 1);

        // Multiple questions
        let r2 = QuestionRequest::new(
            "req-2",
            vec![
                Question::new("Q1", "First?", options.clone()),
                Question::new("Q2", "Second?", options.clone()),
            ],
        );
        let d2 = MultiQuestionDialog::new(r2);
        assert_eq!(d2.tab_count(), 3); // 2 questions + confirm tab
    }

    #[test]
    fn test_question_dialog_tab_navigation() {
        let options = create_test_options();
        let r = QuestionRequest::new(
            "req-1",
            vec![
                Question::new("Q1", "First?", options.clone()),
                Question::new("Q2", "Second?", options.clone()),
            ],
        );
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        assert_eq!(dialog.current_tab(), 0);

        // Next tab
        dialog.next_tab();
        assert_eq!(dialog.current_tab(), 1);

        // Next tab wraps to confirm
        dialog.next_tab();
        assert_eq!(dialog.current_tab(), 2);

        // Next tab wraps to first
        dialog.next_tab();
        assert_eq!(dialog.current_tab(), 0);

        // Previous tab
        dialog.prev_tab();
        assert_eq!(dialog.current_tab(), 2); // Wraps from 0 to 2

        // Select specific tab
        dialog.select_tab(1);
        assert_eq!(dialog.current_tab(), 1);
    }

    #[test]
    fn test_question_dialog_option_navigation() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options.clone())]);
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        assert_eq!(dialog.selected_option(), 0);

        // Next option
        dialog.next_option();
        assert_eq!(dialog.selected_option(), 1);

        dialog.next_option();
        assert_eq!(dialog.selected_option(), 2);

        // Wrap around
        dialog.next_option();
        assert_eq!(dialog.selected_option(), 0);

        // Previous option
        dialog.prev_option();
        assert_eq!(dialog.selected_option(), 2); // Wraps from 0 to 2
    }

    #[test]
    fn test_question_dialog_single_select() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options.clone())]);
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        // Select first option
        dialog.select_option_index(0);
        assert_eq!(dialog.answers()[0], vec!["Option 1"].as_slice());

        // Select second option (replaces first)
        dialog.select_option_index(1);
        assert_eq!(dialog.answers()[0], vec!["Option 2"].as_slice());
    }

    #[test]
    fn test_question_dialog_multi_select_toggle() {
        let options = create_test_options();
        let r = QuestionRequest::new(
            "req-1",
            vec![Question::new("Q1", "Test?", options.clone()).with_multiple(true)],
        );
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        // Toggle first option
        dialog.toggle_option(0);
        assert!(dialog.is_option_selected(0));
        assert!(!dialog.is_option_selected(1));

        // Toggle second option
        dialog.toggle_option(1);
        assert!(dialog.is_option_selected(0));
        assert!(dialog.is_option_selected(1));

        // Toggle first option off
        dialog.toggle_option(0);
        assert!(!dialog.is_option_selected(0));
        assert!(dialog.is_option_selected(1));
    }

    #[test]
    fn test_question_dialog_reject_clears_answers() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options.clone())]);
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        // Select an option
        dialog.select_option_index(0);
        assert_eq!(dialog.answers()[0], vec!["Option 1"].as_slice());

        // Reject
        dialog.reject();
        assert!(!dialog.is_visible());
        assert!(dialog.answers()[0].is_empty());
    }

    #[test]
    fn test_question_dialog_focus_transitions() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options)]);
        let mut dialog = MultiQuestionDialog::new(r);

        assert!(!dialog.is_focused());

        dialog.on_focus();
        assert!(dialog.is_focused());

        dialog.on_blur();
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_question_dialog_is_focusable() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options)]);
        let dialog = MultiQuestionDialog::new(r);
        assert!(dialog.is_focusable());
    }

    #[test]
    fn test_question_dialog_custom_option() {
        let options = vec![QuestionOption::new("Option 1")];
        let r = QuestionRequest::new(
            "req-1",
            vec![Question::new("Q1", "Test?", options.clone()).with_custom(true)],
        );
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        // Should have 2 options (1 regular + custom)
        assert_eq!(dialog.option_count(), 2);

        // Select custom option
        dialog.select_option_index(1);
        assert!(dialog.is_custom_selected());
    }

    #[test]
    fn test_question_dialog_serialization() {
        let option = QuestionOption::new("Test").with_description("A test option");
        let json = serde_json::to_string(&option).unwrap();
        let deserialized: QuestionOption = serde_json::from_str(&json).unwrap();
        assert_eq!(option, deserialized);

        let question = Question::new("Header", "What?", vec![option])
            .with_multiple(true)
            .with_custom(true);
        let json = serde_json::to_string(&question).unwrap();
        let deserialized: Question = serde_json::from_str(&json).unwrap();
        assert_eq!(question, deserialized);
    }

    #[test]
    fn test_question_dialog_handle_input_not_visible() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options)]);
        let mut dialog = MultiQuestionDialog::new(r);
        // Not shown by default

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Continue);
    }

    #[test]
    fn test_question_dialog_handle_input_escape() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options)]);
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_question_dialog_handle_input_navigation() {
        let options = create_test_options();
        let r = QuestionRequest::new(
            "req-1",
            vec![
                Question::new("Q1", "Test?", options.clone()),
                Question::new("Q2", "Second?", options.clone()),
            ],
        );
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        // Test right arrow for next tab
        let event = KeyEvent::new(
            crossterm::event::KeyCode::Right,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert_eq!(dialog.current_tab(), 1);

        // Test down arrow for next option
        let event = KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert_eq!(dialog.selected_option(), 1);

        // Test up arrow for previous option
        let event = KeyEvent::new(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert_eq!(dialog.selected_option(), 0);
    }

    #[test]
    fn test_question_dialog_handle_input_number_keys() {
        let options = create_test_options();
        let r = QuestionRequest::new(
            "req-1",
            vec![Question::new("Q1", "Test?", options).with_multiple(true)],
        );
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        // Press '2' to select second option
        let event = KeyEvent::new(
            crossterm::event::KeyCode::Char('2'),
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        // Should be selected and toggled in multi-select mode
        assert!(dialog.is_option_selected(1));
    }

    #[test]
    fn test_question_dialog_render_does_not_panic() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options)]);
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.show();

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                dialog.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_question_dialog_update() {
        let options = create_test_options();
        let r = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options)]);
        let mut dialog = MultiQuestionDialog::new(r);
        dialog.update(Duration::from_millis(16));
        // Should not panic
    }

    #[test]
    fn test_question_dialog_component_id_unique() {
        let options = create_test_options();
        let r1 = QuestionRequest::new("req-1", vec![Question::new("Q1", "Test?", options.clone())]);
        let r2 = QuestionRequest::new("req-2", vec![Question::new("Q2", "Test?", options)]);
        let dialog1 = MultiQuestionDialog::new(r1);
        let dialog2 = MultiQuestionDialog::new(r2);
        assert_ne!(dialog1.id(), dialog2.id());
    }
}
