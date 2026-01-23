use crate::cli::commands::TerminalArgs;
use crate::services::kernel::KernelManager;
use crate::services::llm::LlmStore;
use crate::services::pty::TerminalManager;
use crate::services::workspace::WorkspaceState;
use crate::services::audit::AuditLog;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, Widget},
    Frame, Terminal,
};
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct TerminalUI {
    kernel: Arc<Mutex<KernelManager>>,
    llm_store: Arc<Mutex<LlmStore>>,
    workspace: PathBuf,
    should_quit: bool,
    input_mode: InputMode,
    current_input: String,
    messages: Vec<String>,
    selected_message: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
    Search,
}

impl TerminalUI {
    pub fn new(
        kernel: Arc<Mutex<KernelManager>>,
        llm_store: Arc<Mutex<LlmStore>>,
        workspace: PathBuf,
    ) -> Self {
        Self {
            kernel,
            llm_store,
            workspace,
            should_quit: false,
            input_mode: InputMode::Normal,
            current_input: String::new(),
            messages: Vec::new(),
            selected_message: 0,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Setup terminal
        enable_raw_mode()?;
        execute!(stdout(), EnableMouseCapture)?;
        execute!(stdout(), Clear(ClearType::All))?;

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.hide_cursor()?;

        // Main loop
        while !self.should_quit {
            terminal.draw(|f| self.ui(f).unwrap())?;
            self.handle_events()?;
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(stdout(), DisableMouseCapture)?;
        terminal.show_cursor()?;
        execute!(stdout(), Clear(ClearType::All))?;

        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) -> Result<(), Box<dyn std::error::Error>> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Header
        let header = Block::default()
            .borders(Borders::ALL)
            .title("TauriHands - AI Development Agent")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .render(f, chunks[0]);

        // Main content
        let main_content = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(chunks[1]);

        // Messages panel
        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let style = if i == self.selected_message {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };
                ListItem::new(msg.as_str()).style(style)
            })
            .collect();

        let messages_panel = Block::default()
            .borders(Borders::ALL)
            .title("Messages")
            .render(
                f,
                main_content[0],
                &mut List::new(messages)
                    .block(Block::default().borders(Borders::ALL).title("Messages"))
                    .style(Style::default().fg(Color::White)),
            );

        // Details panel
        let details_text = if let Some(msg) = self.messages.get(self.selected_message) {
            msg.as_str()
        } else {
            "Select a message to view details"
        };

        let details_panel = Block::default()
            .borders(Borders::ALL)
            .title("Details")
            .render(
                f,
                main_content[1],
                &mut Paragraph::new(details_text)
                    .block(Block::default().borders(Borders::ALL).title("Details"))
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: true }),
            );

        // Input/status bar
        let input_text = match self.input_mode {
            InputMode::Normal => format!("> {}", self.current_input),
            InputMode::Command => format!(":{} ", self.current_input),
            InputMode::Search => format!("/{} ", self.current_input),
        };

        let status_bar = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .render(
                f,
                chunks[2],
                &mut Paragraph::new(input_text.as_str())
                    .style(Style::default().fg(Color::White))
                    .block(Block::default().borders(Borders::ALL)),
            );

        Ok(())
    }

    fn handle_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => self.handle_key_event(key),
                Event::Mouse(mouse) => self.handle_mouse_event(mouse),
                Event::Resize(_, _) => {}
                Event::FocusGained => {}
                Event::FocusLost => {}
                Event::Paste(_) => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) {
        match key.kind {
            KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Char(':') => self.input_mode = InputMode::Command,
                KeyCode::Char('/') => self.input_mode = InputMode::Search,
                KeyCode::Backspace => {
                    self.current_input.pop();
                }
                KeyCode::Enter => self.handle_input(),
                KeyCode::Up => {
                    if self.selected_message > 0 {
                        self.selected_message -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected_message < self.messages.len().saturating_sub(1) {
                        self.selected_message += 1;
                    }
                }
                KeyCode::Char(c) => {
                    self.current_input.push(c);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_mouse_event(&mut self, _mouse: event::MouseEvent) {
        // TODO: Implement mouse interactions
    }

    fn handle_input(&mut self) {
        if !self.current_input.is_empty() {
            match self.input_mode {
                InputMode::Normal => {
                    self.add_message(format!("User: {}", self.current_input));
                    // TODO: Process the input through the kernel
                    self.current_input.clear();
                }
                InputMode::Command => {
                    let input = self.current_input.clone();
                    self.execute_command(&input);
                    self.current_input.clear();
                    self.input_mode = InputMode::Normal;
                }
                InputMode::Search => {
                    let query = self.current_input.clone();
                    self.search_messages(&query);
                    self.current_input.clear();
                    self.input_mode = InputMode::Normal;
                }
            }
        }
    }

    fn add_message(&mut self, message: String) {
        self.messages.push(message);
        if self.messages.len() > 1000 {
            self.messages.remove(0);
        }
    }

    fn execute_command(&mut self, command: &str) {
        match command {
            "quit" | "exit" => self.should_quit = true,
            "clear" => {
                self.messages.clear();
                self.selected_message = 0;
            }
            "help" => {
                self.add_message("Available commands:".to_string());
                self.add_message("  :quit  - Exit the application".to_string());
                self.add_message("  :clear - Clear messages".to_string());
                self.add_message("  :help  - Show this help".to_string());
            }
            _ => {
                self.add_message(format!("Unknown command: {}", command));
            }
        }
    }

    fn search_messages(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }

        for (i, msg) in self.messages.iter().enumerate() {
            if msg.to_lowercase().contains(&query.to_lowercase()) {
                self.selected_message = i;
                break;
            }
        }
    }
}

pub fn start_terminal_mode(args: TerminalArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize kernel and LLM store
    let workspace = args.workspace.unwrap_or_else(|| std::env::current_dir().unwrap());
    let llm_store = Arc::new(Mutex::new(LlmStore::new(workspace.clone())));
    let terminal = TerminalManager::new(workspace.join(".taurihands"));
    let workspace_state = WorkspaceState::new(workspace.clone());
    let audit = AuditLog::new(workspace.join(".taurihands"));
    let kernel = Arc::new(Mutex::new(KernelManager::new(
        workspace.clone(),
        terminal,
        workspace_state,
        audit,
        workspace.join(".taurihands"),
    )));

    let mut terminal_ui = TerminalUI::new(
        Arc::new(Mutex::new(KernelManager::new(
            workspace.clone(),
            TerminalManager::new(workspace.join(".taurihands")),
            WorkspaceState::new(workspace.clone()),
            AuditLog::new(workspace.join(".taurihands")),
            workspace.join(".taurihands"),
        ))),
        llm_store,
        workspace.clone(),
    );
    terminal_ui.run()
}
