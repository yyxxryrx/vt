use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use vt::protocol::ImageProtocol;
use vt::ratatui::{AsyncImageState, AsyncImageWidget, FontSize, ImageWorker};

struct App {
    should_quit: bool,
    protocol: ImageProtocol,
    show_help: bool,
    image_worker: ImageWorker,
    image_state: Option<AsyncImageState>,
}

impl App {
    fn new(protocol: ImageProtocol) -> Self {
        Self {
            should_quit: false,
            protocol,
            show_help: true,
            image_worker: ImageWorker::new(),
            image_state: None,
        }
    }

    fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('h') => self.show_help = !self.show_help,
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let image_path = args.get(1).cloned().unwrap_or_else(|| {
        eprintln!("Usage: ratatui-example <image_path>");
        std::process::exit(1);
    });

    let protocol = vt::protocol::determine_protocol(None);
    let img = image::open(&image_path).unwrap_or_else(|e| {
        eprintln!("Failed to load image: {}", e);
        std::process::exit(1);
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(protocol);
    app.image_state = Some(AsyncImageState::new(img));

    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key.code);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(if app.show_help { 6 } else { 0 }),
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_image(f, app, chunks[1]);
    if app.show_help {
        render_help(f, app, chunks[2]);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let fs = FontSize::from_terminal();
    let title = format!(
        "VT Image Viewer | Protocol: {:?} | Font: {}×{} px | Press h for help, q to quit",
        app.protocol, fs.width, fs.height,
    );
    let header = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default().fg(Color::Cyan),
    )))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, area);
}

fn render_image(f: &mut Frame, app: &mut App, area: Rect) {
    let inner = Block::default()
        .borders(Borders::ALL)
        .title("Image")
        .inner(area);
    f.render_widget(Block::default().borders(Borders::ALL).title("Image"), area);

    if let Some(ref mut state) = app.image_state {
        let widget = AsyncImageWidget::new(&mut app.image_worker, app.protocol, 255);
        f.render_stateful_widget(widget, inner, state);
    }
}

fn render_help(f: &mut Frame, _app: &App, area: Rect) {
    let help = vec![
        Line::from(vec![Span::styled(
            "Key bindings:",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("q/Esc", Style::default().fg(Color::Green)),
            Span::raw("  Quit"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("h", Style::default().fg(Color::Green)),
            Span::raw("     Toggle help"),
        ]),
    ];
    let help_widget =
        Paragraph::new(help).block(Block::default().borders(Borders::TOP).title("Help"));
    f.render_widget(help_widget, area);
}
