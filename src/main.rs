use iced::alignment::{self, Alignment};
use iced::executor;
// use iced::keyboard;
use iced::theme::{self, Theme};
use iced::time;
use iced::widget::pane_grid::{self, Configuration, PaneGrid};
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Application, Command, Element, Length, Settings, Subscription};
use iced_lazy::responsive;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub fn main() -> iced::Result {
    MainWindow::run(Settings::default())
}

struct MainWindow {
    panes: pane_grid::State<AppPane>,
    focus: Option<pane_grid::Pane>,
    scripts_to_run: Vec<String>,
    start_times: Vec<Instant>,
    running_progress: isize,
    progress_receiver: Option<mpsc::Receiver<(isize, Instant)>>,
}

#[derive(Debug, Clone)]
enum Message {
    //FocusAdjacent(pane_grid::Direction),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Maximize(pane_grid::Pane),
    Restore,
    AddScriptToRun(String),
    RunScripts(),
    Tick(Instant),
}

impl Application for MainWindow {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let pane_configuration = Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.6,
            a: Box::new(Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: 0.5,
                a: Box::new(Configuration::Pane(AppPane::new(PaneVariant::ScriptList))),
                b: Box::new(Configuration::Pane(AppPane::new(
                    PaneVariant::ExecutionList,
                ))),
            }),
            b: Box::new(Configuration::Pane(AppPane::new(PaneVariant::LogOutput))),
        };
        let panes = pane_grid::State::with_configuration(pane_configuration);

        (
            MainWindow {
                panes,
                focus: None,
                scripts_to_run: Vec::new(),
                start_times: Vec::new(),
                running_progress: -1,
                progress_receiver: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Scripter")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Message::FocusAdjacent(direction) => {
            //     if let Some(pane) = self.focus {
            //         if let Some(adjacent) = self.panes.adjacent(&pane, direction) {
            //             self.focus = Some(adjacent);
            //         }
            //     }
            // }
            Message::Clicked(pane) => {
                self.focus = Some(pane);
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.swap(&pane, &target);
            }
            Message::Dragged(_) => {}
            Message::Maximize(pane) => self.panes.maximize(&pane),
            Message::Restore => {
                self.panes.restore();
            }
            Message::AddScriptToRun(script) => {
                if self.running_progress == -1 {
                    self.scripts_to_run.push(script);
                }
            }
            Message::RunScripts() => {
                if self.running_progress == -1 {
                    self.running_progress = 0;
                    self.start_times.clear();
                    let (tx, rx) = mpsc::channel();
                    let scripts_to_run = self.scripts_to_run.clone();
                    std::thread::spawn(move || {
                        let mut processed_count = 0;
                        for script in scripts_to_run {
                            tx.send((processed_count, Instant::now())).unwrap();
                            // run script blocking
                            let output = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(&script)
                                .output()
                                .expect(format!("failed to execute script: {}", script).as_str());

                            std::fs::create_dir_all("exec_logs")
                                .expect("failed to create \"exec_logs\" directory");

                            std::fs::write(
                                format!("exec_logs/{}_stdout.log", processed_count),
                                output.stdout,
                            )
                            .expect("failed to write stdout to file");

                            std::fs::write(
                                format!("exec_logs/{}_stderr.log", processed_count),
                                output.stderr,
                            )
                            .expect("failed to write stderr to file");

                            processed_count += 1;
                        }
                        tx.send((processed_count, Instant::now())).unwrap();
                    });
                    self.progress_receiver = Some(rx);
                }
            }
            Message::Tick(_now) => {
                if let Some(rx) = &self.progress_receiver {
                    if let Ok(progress) = rx.try_recv() {
                        self.running_progress = progress.0;
                        if progress.0 == self.start_times.len() as isize {
                            self.start_times.push(progress.1);
                        }
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let focus = self.focus;
        let total_panes = self.panes.len();

        let pane_grid = PaneGrid::new(&self.panes, |id, _pane, is_maximized| {
            let is_focused = focus == Some(id);

            let variant = &self.panes.panes[&id].variant;

            let title = row![match variant {
                PaneVariant::ScriptList => "Scripts",
                PaneVariant::ExecutionList => "Executions",
                PaneVariant::LogOutput => "Log",
            }]
            .spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(view_controls(id, total_panes, is_maximized))
                .padding(10)
                .style(if is_focused {
                    style::title_bar_focused
                } else {
                    style::title_bar_active
                });

            pane_grid::Content::new(responsive(move |_size| {
                view_content(&self.scripts_to_run, &self.start_times, self.running_progress, variant)
            }))
            .title_bar(title_bar)
            .style(if is_focused {
                style::pane_focused
            } else {
                style::pane_active
            })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(1)
        .on_click(Message::Clicked)
        .on_drag(Message::Dragged)
        .on_resize(10, Message::Resized);

        container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // can't find how to process keyboard events and other events at the same time
        // so for now we just process other events
        /*subscription::events_with(|event, status| {
            if let event::Status::Captured = status {
                return None;
            }

            match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    modifiers,
                    key_code,
                }) if modifiers.command() => handle_hotkey(key_code),
                _ => None,
            }
        })*/
        time::every(Duration::from_millis(10)).map(Message::Tick)
    }
}

// fn handle_hotkey(key_code: keyboard::KeyCode) -> Option<Message> {
//     use keyboard::KeyCode;
//     use pane_grid::Direction;
//
//     let direction = match key_code {
//         KeyCode::Up => Some(Direction::Up),
//         KeyCode::Down => Some(Direction::Down),
//         KeyCode::Left => Some(Direction::Left),
//         KeyCode::Right => Some(Direction::Right),
//         _ => None,
//     };
//
//     match key_code {
//         KeyCode::V => Some(Message::SplitFocused(Axis::Vertical)),
//         KeyCode::H => Some(Message::SplitFocused(Axis::Horizontal)),
//         KeyCode::W => Some(Message::CloseFocused),
//         _ => direction.map(Message::FocusAdjacent),
//     }
// }

#[derive(PartialEq)]
enum PaneVariant {
    ScriptList,
    ExecutionList,
    LogOutput,
}

struct AppPane {
    variant: PaneVariant,
}

impl AppPane {
    fn new(variant: PaneVariant) -> Self {
        Self { variant }
    }
}

fn produce_script_list_content<'a>() -> Column<'a, Message> {
    let button = |label, message| {
        button(
            text(label)
                //.width(Length::Fill)
                .vertical_alignment(alignment::Vertical::Center)
                .size(16),
        )
        //.width(Length::Fill)
        .padding(4)
        .on_press(message)
    };

    // iterate over files in "scripts" directory
    let mut files = vec![];
    let dir = std::fs::read_dir("scripts").unwrap();
    for entry in dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }

    let data: Element<_> = column(
        files
            .iter()
            .enumerate()
            .map(|(_i, file)| {
                row![
                    text(
                        file.file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or("[error]")
                            .to_string()
                    ),
                    text(" "),
                    button(
                        "Add",
                        Message::AddScriptToRun(file.to_str().unwrap_or_default().to_string())
                    )
                ]
                .into()
            })
            .collect(),
    )
    .spacing(10)
    .into();

    return column![scrollable(data),]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Center);
}

fn produce_execution_list_content<'a>(
    scripts_to_run: &Vec<String>,
    start_times: &Vec<Instant>,
    progress: isize,
) -> Column<'a, Message> {
    let button = |label, message| {
        button(
            text(label)
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(16),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(message)
    };

    let data: Element<_> = column(
        scripts_to_run
            .iter()
            .enumerate()
            .map(|(i, element)| {
                text(if (i as isize) == progress {
                    if start_times.len() > i {
                        let time_taken_sec = Instant::now().duration_since(start_times[i]).as_secs();
                        format!("[...] {} ({:02}:{:02})", element, time_taken_sec / 60, time_taken_sec % 60)
                    } else {
                        format!("[...] {}", element)
                    }
                } else if (i as isize) < progress {
                    if start_times.len() > i + 1 {
                        let time_taken_sec = start_times[i + 1].duration_since(start_times[i]).as_secs();
                        format!("[DONE] {} ({:02}:{:02})", element, time_taken_sec / 60, time_taken_sec % 60)
                    } else {
                        format!("[DONE] {}", element)
                    }
                } else {
                    format!("{}", element)
                })
                .into()
            })
            .collect(),
    )
    .spacing(10)
    .into();

    let controls = column![button("Run", Message::RunScripts(),),]
        .spacing(5)
        .max_width(150);

    return column![scrollable(data), controls,]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Center);
}

fn produce_log_output_content<'a>() -> Column<'a, Message> {
    let elements = ["line1", "line2", "line3"];
    let data: Element<_> = column(
        elements
            .iter()
            .enumerate()
            .map(|(_i, element)| text(element).into())
            .collect(),
    )
    .spacing(10)
    .into();

    return column![scrollable(data),]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Center);
}

fn view_content<'a>(
    scripts_to_run: &Vec<String>,
    start_times: &Vec<Instant>,
    progress: isize,
    variant: &PaneVariant,
) -> Element<'a, Message> {
    let content = match variant {
        PaneVariant::ScriptList => produce_script_list_content(),
        PaneVariant::ExecutionList => produce_execution_list_content(scripts_to_run, start_times, progress),
        PaneVariant::LogOutput => produce_log_output_content(),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(5)
        .center_y()
        .into()
}

fn view_controls<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_maximized: bool,
) -> Element<'a, Message> {
    let mut row = row![].spacing(5);

    if total_panes > 1 {
        let toggle = {
            let (content, message) = if is_maximized {
                ("Restore", Message::Restore)
            } else {
                ("Maximize", Message::Maximize(pane))
            };
            button(text(content).size(14))
                .style(theme::Button::Secondary)
                .padding(3)
                .on_press(message)
        };

        row = row.push(toggle);
    }

    row.into()
}

mod style {
    use iced::widget::container;
    use iced::Theme;

    pub fn title_bar_active(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.primary.strong.text),
            background: Some(palette.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane_active(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            background: Some(palette.background.weak.color.into()),
            border_width: 2.0,
            border_color: palette.background.strong.color,
            ..Default::default()
        }
    }

    pub fn pane_focused(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            background: Some(palette.background.weak.color.into()),
            border_width: 2.0,
            border_color: palette.primary.strong.color,
            ..Default::default()
        }
    }
}
