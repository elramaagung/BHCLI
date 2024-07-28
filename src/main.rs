mod bhc;
mod lechatphp;
mod util;
use crate::lechatphp::LoginErr;
use anyhow::{anyhow, Context};
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use clap::Parser;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use std::collections::HashSet;
use colors_transform::{Color, Rgb};
use crossbeam_channel::{self, after, select};
use crossterm::event;
use crossterm::event::Event as CEvent;
use crossterm::event::{MouseEvent, MouseEventKind};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lazy_static::lazy_static;
use linkify::LinkFinder;
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;
use reqwest::blocking::multipart;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use rodio::{source::Source, Decoder, OutputStream};
use select::document::Document;
use select::predicate::{Attr, Name};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::io::{self, Write};
use std::process::Command;
use std::sync::Mutex;
use std::sync::{Arc, MutexGuard};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tui::layout::Rect;
use tui::style::Color as tuiColor;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;
use util::StatefulList;

const LANG: &str = "en";
const SEND_TO_ALL: &str = "s *";
const SEND_TO_MEMBERS: &str = "s ?";
static mut BOT_ACTIVE: bool = false;
const SEND_TO_STAFFS: &str = "s %";
const SEND_TO_ADMINS: &str = "s _";
const SOUND1: &[u8] = include_bytes!("sound1.mp3");
static BAN_IMPOSTERS: bool = true;
const XPLDAN: &str = "XplDan";
const DKF_URL: &str = "http://dkforestseeaaq2dqz2uflmlsybvnq2irzn4ygyvu53oazyorednviid.onion";
const SERVER_DOWN_500_ERR: &str = "500 Internal Server Error, server down";
const SERVER_DOWN_ERR: &str = "502 Bad Gateway, server down";
const KICKED_ERR: &str = "You have been kicked";
const REG_ERR: &str = "This nickname is a registered member";
const NICKNAME_ERR: &str = "Invalid nickname";
const CAPTCHA_WG_ERR: &str = "Wrong Captcha";
const CAPTCHA_FAILED_SOLVE_ERR: &str = "Failed solve captcha";
const CAPTCHA_USED_ERR: &str = "Captcha already used or timed out";
const UNKNOWN_ERR: &str = "Unknown error";
const DNMX_URL: &str = "http://hxuzjtocnzvv5g2rtg2bhwkcbupmk7rclb6lly3fo4tvqkk5oyrv3nid.onion";
const BHCLI_BLOG_URL: &str = "sss";


lazy_static! {
    static ref WARNED_USERS: Mutex<HashMap<String, u32>> = Mutex::new(HashMap::new());
    static ref META_REFRESH_RGX: Regex = Regex::new(r#"url='([^']+)'"#).unwrap();
    static ref SESSION_RGX: Regex = Regex::new(r#"session=([^&]+)"#).unwrap();
    static ref COLOR_RGX: Regex = Regex::new(r#"color:\s*([#\w]+)\s*;"#).unwrap();
    static ref COLOR1_RGX: Regex = Regex::new(r#"^#([0-9A-Fa-f]{6})$"#).unwrap();
    static ref PM_RGX: Regex = Regex::new(r#"^/pm ([^\s]+) (.*)"#).unwrap();
    static ref KICK_RGX: Regex = Regex::new(r#"^/(?:kick|k) ([^\s]+)\s?(.*)"#).unwrap();
    static ref IGNORE_RGX: Regex = Regex::new(r#"^/ignore ([^\s]+)"#).unwrap();
    static ref UNIGNORE_RGX: Regex = Regex::new(r#"^/unignore ([^\s]+)"#).unwrap();
    static ref DLX_RGX: Regex = Regex::new(r#"^/dl([\d]+)$"#).unwrap();
    static ref UPLOAD_RGX: Regex = Regex::new(r#"^/u\s([^\s]+)\s?(?:@([^\s]+)\s)?(.*)$"#).unwrap();
    static ref FIND_RGX: Regex = Regex::new(r#"^/f\s(.*)$"#).unwrap();
    static ref NEW_NICKNAME_RGX: Regex = Regex::new(r#"^/nick\s(.*)$"#).unwrap();
    static ref NEW_COLOR_RGX: Regex = Regex::new(r#"^/color\s(.*)$"#).unwrap();
}

fn default_empty_str() -> String {
    "".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct Profile {
    username: String,
    password: String,
    #[serde(default = "default_empty_str")]
    url: String,
    #[serde(default = "default_empty_str")]
    date_format: String,
    #[serde(default = "default_empty_str")]
    page_php: String,
    #[serde(default = "default_empty_str")]
    members_tag: String,
    #[serde(default = "default_empty_str")]
    keepalive_send_to: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct MyConfig {
    dkf_api_key: Option<String>,
    profiles: HashMap<String, Profile>,
}

#[derive(Parser)]
#[command(name = "bhcli")]
#[command(author = "XplDan <Xpldan@protonmail.com>")]
#[command(version = "0.1.0")]

struct Opts {
    #[arg(long, env = "DKF_API_KEY")]
    dkf_api_key: Option<String>,
    #[arg(short, long, env = "BHC_USERNAME")]
    username: Option<String>,
    #[arg(short, long, env = "BHC_PASSWORD")]
    password: Option<String>,
    #[arg(short, long, env = "BHC_MANUAL_CAPTCHA")]
    manual_captcha: bool,
    #[arg(short, long, env = "BHC_GUEST_COLOR")]
    guest_color: Option<String>,
    #[arg(short, long, env = "BHC_REFRESH_RATE", default_value = "5")]
    refresh_rate: u64,
    #[arg(long, env = "BHC_MAX_LOGIN_RETRY", default_value = "5")]
    max_login_retry: isize,
    #[arg(long)]
    url: Option<String>,
    #[arg(long)]
    page_php: Option<String>,
    #[arg(long)]
    datetime_fmt: Option<String>,
    #[arg(long)]
    members_tag: Option<String>,
    #[arg(short, long)]
    dan: bool,
    #[arg(
        short,
        long,
        env = "BHC_PROXY_URL",
        default_value = "socks5h://127.0.0.1:9050"
    )]
    socks_proxy_url: String,
    #[arg(long)]
    no_proxy: bool,
    #[arg(long, env = "DNMX_USERNAME")]
    dnmx_username: Option<String>,
    #[arg(long, env = "DNMX_PASSWORD")]
    dnmx_password: Option<String>,
    #[arg(short = 'c', long, default_value = "default")]
    profile: String,

    //Strange
    #[arg(long,default_value = "0")]
    keepalive_send_to: Option<String>,

    #[arg(long)]
    session: Option<String>,

    #[arg(long)]
    sxiv: bool,
}

struct LeChatPHPConfig {
    url: String,
    datetime_fmt: String,
    page_php: String,
    keepalive_send_to: String,
    members_tag: String,
    staffs_tag: String,
}

impl LeChatPHPConfig {
    fn new_black_hat_chat_config() -> Self {
        Self {
            url: "http://blkh4ylofapg42tj6ht565klld5i42dhjtysvsnnswte4xt4uvnfj5qd.onion".to_owned(),
            datetime_fmt: "%m-%d %H:%M:%S".to_owned(),
            page_php: "chat.php".to_owned(),
            keepalive_send_to: "0".to_owned(),
            members_tag: "[M] ".to_owned(),
            staffs_tag: "[Staff] ".to_owned(),
        }
    }
}
struct BaseClient {
    username: String,
    password: String,
}

struct LeChatPHPClient {
    base_client: BaseClient,
    guest_color: String,
    client: Client,
    session: Option<String>,
    config: LeChatPHPConfig,
    last_key_event: Option<KeyCode>,
    manual_captcha: bool,
    sxiv: bool,
    refresh_rate: u64,
    max_login_retry: isize,

    is_muted: Arc<Mutex<bool>>,
    show_sys: bool,
    display_guest_view: bool,
    display_member_view: bool,
    display_hidden_msgs: bool,
    tx: crossbeam_channel::Sender<PostType>,
    rx: Arc<Mutex<crossbeam_channel::Receiver<PostType>>>,

    color_tx: crossbeam_channel::Sender<()>,
    color_rx: Arc<Mutex<crossbeam_channel::Receiver<()>>>,
}


impl LeChatPHPClient {
    fn run_forever(&mut self) {
        let max_retry = self.max_login_retry;
        let mut attempt = 0;
        loop {
            match self.login() {
                Err(e) => match e {
                    LoginErr::KickedErr
                    | LoginErr::RegErr
                    | LoginErr::NicknameErr
                    | LoginErr::UnknownErr => {
                        log::error!("{}", e);
                        println!("Login error: {}", e); // Print error message
                        break;
                    }
                    LoginErr::CaptchaFailedSolveErr => {
                        log::error!("{}", e);
                        println!("Captcha failed to solve: {}", e); // Print error message
                        continue;
                    }
                    LoginErr::CaptchaWgErr | LoginErr::CaptchaUsedErr => {}
                    LoginErr::ServerDownErr | LoginErr::ServerDown500Err => {
                        log::error!("{}", e);
                        println!("Server is down: {}", e); // Print error message
                    }
                    LoginErr::Reqwest(err) => {
                        if err.is_connect() {
                            log::error!("{}\nIs tor proxy enabled ?", err);
                            println!("Connection error: {}\nIs tor proxy enabled ?", err); // Print error message
                            break;
                        } else if err.is_timeout() {
                            log::error!("timeout: {}", err);
                            println!("Timeout error: {}", err); // Print error message
                        } else {
                            log::error!("{}", err);
                            println!("Reqwest error: {}", err); // Print error message
                        }
                    }
                },

                Ok(()) => {
                    attempt = 0;
                    match self.get_msgs() {
                        Ok(ExitSignal::NeedLogin) => {}
                        Ok(ExitSignal::Terminate) => return,
                        Err(e) => log::error!("{:?}", e),
                    }
                }
            }
            attempt += 1;
            if max_retry > 0 && attempt > max_retry {
                break;
            }
            self.session = None;
            let retry_in = Duration::from_secs(2);
            let mut msg = format!("retry login in {:?}, attempt: {}", retry_in, attempt);
            if max_retry > 0 {
                msg += &format!("/{}", max_retry);
            }
            println!("{}", msg);
            thread::sleep(retry_in);
        }
    }

    fn start_keepalive_thread(
        &self,
        exit_rx: crossbeam_channel::Receiver<ExitSignal>,
        last_post_rx: crossbeam_channel::Receiver<()>,
    ) -> thread::JoinHandle<()> {
        let tx = self.tx.clone();
        // let msg_actived_bot = format!(">>> [color=#ffffff]Dantca patch update on system >> ..... configuration successful.. not error report > - < actived[/color] <<< |3 min removed |");
        // tx.send(PostType::Post(msg_actived_bot.to_owned(), Some(SEND_TO_ALL.to_owned()))).unwrap();
        let send_to = self.config.keepalive_send_to.clone();
        thread::spawn(move || loop {
            let keep_msg = || {
                let msg_keep = "[color=#ffffff]>>> H-E-L-L-O C-H-A-T-T-E-R-S W-E-L-C-O-M-E B-A-C-K TO BHC <<<[/color]
                Keep it legal and enjoy your stay. 
                You can try !-rules && !-help before. Please follow the !-rules
                                       (Auto message)";
                tx.send(PostType::Post(msg_keep.to_owned(), Some(SEND_TO_ALL.to_owned()))).unwrap();
            };
            let clb = || {
                tx.send(PostType::Post("<keepalive>".to_owned(), Some(send_to.clone())))
                    .unwrap();
                tx.send(PostType::DeleteLast).unwrap();
            };
            let timeout = after(Duration::from_secs(60 * 75));
            select! {
                // Ketika kita mengirim pesan ke server chat,
                // kita akan menerima pesan pada channel ini
                // dan mereset timer untuk keepalive berikutnya.
                recv(&last_post_rx) -> _ => {},
                recv(&exit_rx) -> _ => return,
                recv(&timeout) -> _ => { 
                    keep_msg(); 
                    clb();
                },
            }
        })
    }


    //erver
    fn start_post_msg_thread(
        &self,
        exit_rx: crossbeam_channel::Receiver<ExitSignal>,
        last_post_tx: crossbeam_channel::Sender<()>,
    ) -> thread::JoinHandle<()> {
        let client = self.client.clone();
        let rx = Arc::clone(&self.rx);
        let full_url = format!("{}/{}", &self.config.url, &self.config.page_php);
        let session = self.session.clone().unwrap();
        let url = format!("{}?action=post&session={}", &full_url, &session);
        thread::spawn(move || loop {
            // select! macro fucks all the LSP, therefore the code gymnastic here
            let clb = |v: Result<PostType, crossbeam_channel::RecvError>| match v {
                Ok(post_type_recv) => post_msg(
                    &client,
                    post_type_recv,
                    &full_url,
                    session.clone(),
                    &url,
                    &last_post_tx,
                ),
                Err(_) => return,
            };
            let rx = rx.lock().unwrap();
            select! {
                recv(&exit_rx) -> _ => return,
                recv(&rx) -> v => clb(v),
            }
        })
    }

    // Thread that update messages every "refresh_rate"
    fn start_get_msgs_thread(
        &self,
        sig: &Arc<Mutex<Sig>>,
        messages: &Arc<Mutex<Vec<Message>>>,
        users: &Arc<Mutex<Users>>,
        messages_updated_tx: crossbeam_channel::Sender<()>,
        tx: crossbeam_channel::Sender<PostType>,
    ) -> thread::JoinHandle<()> {
        let client = self.client.clone();
        let messages = Arc::clone(messages);
        let users = Arc::clone(users);
        let session = self.session.clone().unwrap();
        let username = self.base_client.username.clone();
        let refresh_rate = self.refresh_rate;
        let base_url = self.config.url.clone();
        let page_php = self.config.page_php.clone();
        let datetime_fmt = self.config.datetime_fmt.clone();
        let is_muted = Arc::clone(&self.is_muted);
        let exit_rx = sig.lock().unwrap().clone();
        let sig = Arc::clone(sig);
        let members_tag = self.config.members_tag.clone();
        thread::spawn(move || loop {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let source = Decoder::new_mp3(Cursor::new(SOUND1)).unwrap();
            let mut should_notify = false;
            if let Err(err) = get_msgs(
                &client,
                &base_url,
                &page_php,
                &session,
                &username,
                &users,
                &sig,
                &messages_updated_tx,
                &members_tag,
                &datetime_fmt,
                &tx,
                &messages,
                &mut should_notify,
            ) {
                log::error!("{}", err);
            };

            let muted = { *is_muted.lock().unwrap() };
            if should_notify && !muted {
                if let Err(err) = stream_handle.play_raw(source.convert_samples()) {
                    log::error!("{}", err);
                }
            }

            let timeout = after(Duration::from_secs(refresh_rate));
            select! {
                recv(&exit_rx) -> _ => return,
                recv(&timeout) -> _ => {},
            }
        })
    }

    fn get_msgs(&mut self) -> anyhow::Result<ExitSignal> {
        let terminate_signal: ExitSignal;

        let messages: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
        let users: Arc<Mutex<Users>> = Arc::new(Mutex::new(Users::default()));

        // Create default app state
        let mut app = App::default();

        // Each threads gets a clone of the receiver.
        // When someone calls ".signal", all threads receive it,
        // and knows that they have to terminate.
        let sig = Arc::new(Mutex::new(Sig::new()));

        let (messages_updated_tx, messages_updated_rx) = crossbeam_channel::unbounded();
        let (last_post_tx, last_post_rx) = crossbeam_channel::unbounded();

        let h1 = self.start_keepalive_thread(sig.lock().unwrap().clone(), last_post_rx);
        let h2 = self.start_post_msg_thread(sig.lock().unwrap().clone(), last_post_tx);
        let h3 = self.start_get_msgs_thread(&sig, &messages, &users, messages_updated_tx.clone(), self.tx.clone());

        // Terminal initialization
        let mut stdout = io::stdout();
        enable_raw_mode().unwrap();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Setup event handlers
        let (events, h4) = Events::with_config(Config {
            messages_updated_rx,
            exit_rx: sig.lock().unwrap().clone(),
            tick_rate: Duration::from_millis(250),
        });

        loop {
            app.is_muted = *self.is_muted.lock().unwrap();
            app.show_sys = self.show_sys;
            app.display_guest_view = self.display_guest_view;
            app.display_member_view = self.display_member_view;
            app.display_hidden_msgs = self.display_hidden_msgs;
            app.members_tag = self.config.members_tag.clone();
            app.staffs_tag = self.config.staffs_tag.clone();

            // process()
            // Draw UI
            terminal.draw(|f| {
                draw_terminal_frame(f, &mut app, &messages, &users, &self.base_client.username);
            })?;

            // Handle input
            match self.handle_input(&events, &mut app, &messages, &users) {
                Err(ExitSignal::Terminate) => {
                    terminate_signal = ExitSignal::Terminate;
                    sig.lock().unwrap().signal(&terminate_signal);
                    break;
                }
                Err(ExitSignal::NeedLogin) => {
                    terminate_signal = ExitSignal::NeedLogin;
                    sig.lock().unwrap().signal(&terminate_signal);
                    break;
                }
                Ok(_) => continue,
            };
        }

        // Cleanup before leaving
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        terminal.clear()?;
        terminal.set_cursor(0, 0)?;

        h1.join().unwrap();
        h2.join().unwrap();
        h3.join().unwrap();
        h4.join().unwrap();

        Ok(terminate_signal)
    }

    fn post_msg(&self, post_type: PostType) -> anyhow::Result<()> {
        self.tx.send(post_type)?;
        Ok(())
    }

    fn login(&mut self) -> Result<(), LoginErr> {
        // If we provided a session, skip login process
        if self.session.is_some() {
            // println!("Session in params: {:?}", self.session); 
            return Ok(());
        }
        // println!("self.session is not Some");
        // println!("self.sxiv = {:?}", self.sxiv);
        self.session = Some(lechatphp::login(
            &self.client,
            &self.config.url,
            &self.config.page_php,
            &self.base_client.username,
            &self.base_client.password,
            &self.guest_color,
            self.manual_captcha,
            self.sxiv,
        )?);
        Ok(())
    }

    fn logout(&mut self) -> anyhow::Result<()> {
        if let Some(session) = &self.session {
            lechatphp::logout(
                &self.client,
                &self.config.url,
                &self.config.page_php,
                session,
            )?;
            self.session = None;
        }
        Ok(())
    }


    fn start_cycle(&self, color_only: bool) {
        let username = self.base_client.username.clone();
        let tx = self.tx.clone();
        let color_rx = Arc::clone(&self.color_rx);
        thread::spawn(move || {
            let mut idx = 0;
            let colors = [
                "#ff3366", "#ff6633", "#FFCC33", "#33FF66", "#33FFCC", "#33CCFF", "#3366FF",
                "#6633FF", "#CC33FF", "#efefef",
            ];
            loop {
                let color_rx = color_rx.lock().unwrap();
                let timeout = after(Duration::from_millis(5200));
                select! {
                    recv(&color_rx) -> _ => break,
                    recv(&timeout) -> _ => {}
                }
                idx = (idx + 1) % colors.len();
                let color = colors[idx].to_owned();
                if !color_only {
                    let name = format!("{}{}", username, random_string(14));
                    log::error!("New name : {}", name);
                    tx.send(PostType::Profile(color, name)).unwrap();
                } else {
                    tx.send(PostType::NewColor(color)).unwrap();
                }
                // tx.send(PostType::Post("!up".to_owned(), Some(username.clone())))
                //     .unwrap();
                // tx.send(PostType::DeleteLast).unwrap();
            }
            let msg = PostType::Profile("#90ee90".to_owned(), username);
            tx.send(msg).unwrap();
        });
    }

    fn handle_input(
        &mut self,
        events: &Events,
        app: &mut App,
        messages: &Arc<Mutex<Vec<Message>>>,
        users: &Arc<Mutex<Users>>,
    ) -> Result<(), ExitSignal> {
        match events.next() {
            Ok(Event::NeedLogin) => return Err(ExitSignal::NeedLogin),
            Ok(Event::Terminate) => return Err(ExitSignal::Terminate),
            Ok(Event::Input(evt)) => self.handle_event(app, messages, users, evt),
            _ => Ok(()),
        }
    }

    fn handle_event(
        &mut self,
        app: &mut App,
        messages: &Arc<Mutex<Vec<Message>>>,
        users: &Arc<Mutex<Users>>,
        event: event::Event,
    ) -> Result<(), ExitSignal> {
        match event {
            event::Event::Resize(_cols, _rows) => Ok(()),
            event::Event::FocusGained => Ok(()),
            event::Event::FocusLost => Ok(()),
            event::Event::Paste(_) => Ok(()),
            event::Event::Key(key_event) => self.handle_key_event(app, messages, users, key_event),
            event::Event::Mouse(mouse_event) => self.handle_mouse_event(app, mouse_event),
        }
    }

    fn handle_key_event(
        &mut self,
        app: &mut App,
        messages: &Arc<Mutex<Vec<Message>>>,
        users: &Arc<Mutex<Users>>,
        key_event: KeyEvent,
    ) -> Result<(), ExitSignal> {
        if app.input_mode != InputMode::Normal {
            self.last_key_event = None;
        }
        match app.input_mode {
            InputMode::LongMessage => {
                self.handle_long_message_mode_key_event(app, key_event, messages)
            }
            InputMode::Normal => self.handle_normal_mode_key_event(app, key_event, messages),
            InputMode::Editing | InputMode::EditingErr => {
                self.handle_editing_mode_key_event(app, key_event, users)
            }
        }
    }

    fn handle_long_message_mode_key_event(
        &mut self,
        app: &mut App,
        key_event: KeyEvent,
        messages: &Arc<Mutex<Vec<Message>>>,
    ) -> Result<(), ExitSignal> {
        match key_event {
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            }
            | KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_long_message_mode_key_event_esc(app),
            KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_long_message_mode_key_event_ctrl_d(app, messages),
            _ => {}
        }
        Ok(())
    }

    fn handle_normal_mode_key_event(
        &mut self,
        app: &mut App,
        key_event: KeyEvent,
        messages: &Arc<Mutex<Vec<Message>>>,
    ) -> Result<(), ExitSignal> {
        match key_event {
            KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_slash(app),
            KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
                ..
            }
            | KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_down(app),
            KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
                ..
            }
            | KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_up(app),
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_enter(app, messages),
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_backspace(app, messages),
            KeyEvent {
                code: KeyCode::Char('y'),
                modifiers: KeyModifiers::NONE,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_normal_mode_key_event_yank(app),
            KeyEvent {
                code: KeyCode::Char('Y'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_yank_link(app),

            //Strange
            KeyEvent {
                code: KeyCode::Char('D'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_download_link(app),

            //Strange
            KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_download_and_view(app),

            // KeyEvent {
            //     code: KeyCode::Char('d'),
            //     modifiers: KeyModifiers::NONE,
            //     ..
            // } => self.handle_normal_mode_key_event_debug(app),
            // KeyEvent {
            //     code: KeyCode::Char('D'),
            //     modifiers: KeyModifiers::SHIFT,
            //     ..
            // } => self.handle_normal_mode_key_event_debug2(app),
            KeyEvent {
                code: KeyCode::Char('m'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_toggle_mute(),
            KeyEvent {
                code: KeyCode::Char('S'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_toggle_sys(),
            KeyEvent {
                code: KeyCode::Char('M'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_toggle_member_view(),
            KeyEvent {
                code: KeyCode::Char('G'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_toggle_guest_view(),
            KeyEvent {
                code: KeyCode::Char('H'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_toggle_hidden(),
            KeyEvent {
                code: KeyCode::Char('i'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_input_mode(app),
            KeyEvent {
                code: KeyCode::Char('Q'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_logout()?,
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_exit()?,
            KeyEvent {
                code: KeyCode::Char('t'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_tag(app),
            KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_pm(app),
            KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_normal_mode_key_event_kick(app),
            KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_normal_mode_key_event_warn(app),
            KeyEvent {
                code: KeyCode::Char('T'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_page_up(app),
            KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }
            | KeyEvent {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_page_down(app),
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_esc(app),
            KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_normal_mode_key_event_shift_u(app),
            KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_normal_mode_key_event_g(app),
            _ => {}
        }
        self.last_key_event = Some(key_event.code);
        Ok(())
    }

    fn handle_editing_mode_key_event(
        &mut self,
        app: &mut App,
        key_event: KeyEvent,
        users: &Arc<Mutex<Users>>,
    ) -> Result<(), ExitSignal> {
        app.input_mode = InputMode::Editing;
        match key_event {
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_enter(app)?,
            KeyEvent {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_tab(app, users),
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_editing_mode_key_event_ctrl_c(app),
            KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_editing_mode_key_event_ctrl_a(app),
            KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_editing_mode_key_event_ctrl_e(app),
            KeyEvent {
                code: KeyCode::Char('f'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_editing_mode_key_event_ctrl_f(app),
            KeyEvent {
                code: KeyCode::Char('b'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_editing_mode_key_event_ctrl_b(app),
            KeyEvent {
                code: KeyCode::Char('v'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_editing_mode_key_event_ctrl_v(app),
            KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_left(app),
            KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_right(app),
            KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_down(app),
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::SHIFT,
                ..
            } => self.handle_editing_mode_key_event_shift_c(app, c),
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_backspace(app),
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_delete(app),
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_editing_mode_key_event_esc(app),
            _ => {}
        }
        Ok(())
    }

    fn handle_long_message_mode_key_event_esc(&mut self, app: &mut App) {
        app.long_message = None;
        app.input_mode = InputMode::Normal;
    }

    fn handle_long_message_mode_key_event_ctrl_d(
        &mut self,
        app: &mut App,
        messages: &Arc<Mutex<Vec<Message>>>,
    ) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(item) = app.items.items.get(idx) {
                self.post_msg(PostType::Clean(item.date.to_owned(), item.text.text()))
                    .unwrap();
                let mut messages = messages.lock().unwrap();
                if let Some(pos) = messages
                    .iter()
                    .position(|m| m.date == item.date && m.text == item.text)
                {
                    messages[pos].hide = !messages[pos].hide;
                }
                app.long_message = None;
                app.input_mode = InputMode::Normal;
            }
        }
    }

    fn handle_normal_mode_key_event_up(&mut self, app: &mut App) {
        app.items.previous()
    }

    fn handle_normal_mode_key_event_down(&mut self, app: &mut App) {
        app.items.next()
    }

    fn handle_normal_mode_key_event_slash(&mut self, app: &mut App) {
        app.items.unselect();
        app.input = "/".to_owned();
        app.input_idx = app.input.width();
        app.input_mode = InputMode::Editing;
    }

    fn handle_normal_mode_key_event_enter(
        &mut self,
        app: &mut App,
        messages: &Arc<Mutex<Vec<Message>>>,
    ) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(item) = app.items.items.get(idx) {
                // If we have a filter, <enter> will "jump" to the message
                if !app.filter.is_empty() {
                    let idx = messages
                        .lock()
                        .unwrap()
                        .iter()
                        .enumerate()
                        .find(|(_, e)| e.date == item.date)
                        .map(|(i, _)| i);
                    app.clear_filter();
                    app.items.state.select(idx);
                    return;
                }
                app.long_message = Some(item.clone());
                app.input_mode = InputMode::LongMessage;
            }
        }
    }

    fn handle_normal_mode_key_event_backspace(
        &mut self,
        app: &mut App,
        messages: &Arc<Mutex<Vec<Message>>>,
    ) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(item) = app.items.items.get(idx) {
                let mut messages = messages.lock().unwrap();
                if let Some(pos) = messages
                    .iter()
                    .position(|m| m.date == item.date && m.text == item.text)
                {
                    if item.deleted {
                        messages.remove(pos);
                    } else {
                        messages[pos].hide = !messages[pos].hide;
                    }
                }
            }
        }
    }

    fn handle_normal_mode_key_event_yank(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(item) = app.items.items.get(idx) {
                if let Some(upload_link) = &item.upload_link {
                    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                    let mut out = format!("{}{}", self.config.url, upload_link);
                    if let Some((_, _, msg)) = get_message(&item.text, &self.config.members_tag) {
                        out = format!("{} {}", msg, out);
                    }
                    ctx.set_contents(out).unwrap();
                } else if let Some((_, _, msg)) = get_message(&item.text, &self.config.members_tag)
                {
                    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                    ctx.set_contents(msg).unwrap();
                }
            }
        }
    }

    fn handle_normal_mode_key_event_yank_link(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(item) = app.items.items.get(idx) {
                if let Some(upload_link) = &item.upload_link {
                    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                    let out = format!("{}{}", self.config.url, upload_link);
                    ctx.set_contents(out).unwrap();
                } else if let Some((_, _, msg)) = get_message(&item.text, &self.config.members_tag)
                {
                    let finder = LinkFinder::new();
                    let links: Vec<_> = finder.links(msg.as_str()).collect();
                    if let Some(link) = links.get(0) {
                        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                        ctx.set_contents(link.as_str().to_owned()).unwrap();
                    }
                }
            }
        }
    }

    //Strange
    fn handle_normal_mode_key_event_download_link(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(item) = app.items.items.get(idx) {
                if let Some(upload_link) = &item.upload_link {
                    let url = format!("{}{}", self.config.url, upload_link);
                    let _ = Command::new("curl")
                        .args([
                            "--socks5",
                            "localhost:9050",
                            "--socks5-hostname",
                            "localhost:9050",
                            &url,
                        ])
                        .arg("-o")
                        .arg("download.img")
                        .output()
                        .expect("Failed to execute curl command");
                } else if let Some((_, _, msg)) = get_message(&item.text, &self.config.members_tag)
                {
                    let finder = LinkFinder::new();
                    let links: Vec<_> = finder.links(msg.as_str()).collect();
                    if let Some(link) = links.first() {
                        let url = link.as_str();
                        let _ = Command::new("curl")
                            .args([
                                "--socks5",
                                "localhost:9050",
                                "--socks5-hostname",
                                "localhost:9050",
                                url,
                            ])
                            .arg("-o")
                            .arg("download.img")
                            .output()
                            .expect("Failed to execute curl command");
                    }
                }
            }
        }
    }

    //strageEdit
    fn handle_normal_mode_key_event_download_and_view(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(item) = app.items.items.get(idx) {
                if let Some(upload_link) = &item.upload_link {
                    let url = format!("{}{}", self.config.url, upload_link);
                    let _ = Command::new("curl")
                        .args([
                            "--socks5",
                            "localhost:9050",
                            "--socks5-hostname",
                            "localhost:9050",
                            &url,
                        ])
                        .arg("-o")
                        .arg("download.img")
                        .output()
                        .expect("Failed to execute curl command");

                    let _ = Command::new("xdg-open")
                        .arg("./download.img")
                        .output()
                        .expect("Failed to execute sxiv command");
                } else if let Some((_, _, msg)) = get_message(&item.text, &self.config.members_tag)
                {
                    let finder = LinkFinder::new();
                    let links: Vec<_> = finder.links(msg.as_str()).collect();
                    if let Some(link) = links.first() {
                        let url = link.as_str();
                        let _ = Command::new("curl")
                            .args([
                                "--socks5",
                                "localhost:9050",
                                "--socks5-hostname",
                                "localhost:9050",
                                url,
                            ])
                            .arg("-o")
                            .arg("download.img")
                            .output()
                            .expect("Failed to execute curl command");

                        let _ = Command::new("sxiv")
                            .arg("./download.img")
                            .output()
                            .expect("Failed to execute sxiv command");
                    }
                }
            }
        }
    }

    fn handle_normal_mode_key_event_toggle_mute(&mut self) {
        let mut is_muted = self.is_muted.lock().unwrap();
        *is_muted = !*is_muted;
    }

    fn handle_normal_mode_key_event_toggle_sys(&mut self) {
        self.show_sys = !self.show_sys;
    }

    fn handle_normal_mode_key_event_toggle_guest_view(&mut self) {
        self.display_guest_view = !self.display_guest_view;
    }

    fn handle_normal_mode_key_event_toggle_member_view(&mut self) {
        self.display_member_view = !self.display_member_view;
    }

    fn handle_normal_mode_key_event_g(&mut self, app: &mut App) {
        // Handle "gg" key combination
        if self.last_key_event == Some(KeyCode::Char('g')) {
            app.items.select_top();
            self.last_key_event = None;
        }
    }

    fn handle_normal_mode_key_event_toggle_hidden(&mut self) {
        self.display_hidden_msgs = !self.display_hidden_msgs;
    }

    fn handle_normal_mode_key_event_input_mode(&mut self, app: &mut App) {
        app.input_mode = InputMode::Editing;
        app.items.unselect();
    }

    fn handle_normal_mode_key_event_logout(&mut self) -> Result<(), ExitSignal> {
        self.logout().unwrap();
        return Err(ExitSignal::Terminate);
    }

    fn handle_normal_mode_key_event_exit(&mut self) -> Result<(), ExitSignal> {
        return Err(ExitSignal::Terminate);
    }

    fn handle_normal_mode_key_event_tag(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            let text = &app.items.items.get(idx).unwrap().text;
            if let Some(username) =
                get_username(&self.base_client.username, &text, &self.config.members_tag)
            {
                if text.text().starts_with(&app.members_tag) {
                    app.input = format!("/m @{} ", username);
                } else {
                    app.input = format!("@{} ", username);
                }
                app.input_idx = app.input.width();
                app.input_mode = InputMode::Editing;
                app.items.unselect();
            }
        }
    }

    fn handle_normal_mode_key_event_pm(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(username) = get_username(
                &self.base_client.username,
                &app.items.items.get(idx).unwrap().text,
                &self.config.members_tag,
            ) {
                app.input = format!("/pm {} ", username);
                app.input_idx = app.input.width();
                app.input_mode = InputMode::Editing;
                app.items.unselect();
            }
        }
    }

    fn handle_normal_mode_key_event_kick(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(username) = get_username(
                &self.base_client.username,
                &app.items.items.get(idx).unwrap().text,
                &self.config.members_tag,
            ) {
                app.input = format!("/kick {} ", username);
                app.input_idx = app.input.width();
                app.input_mode = InputMode::Editing;
                app.items.unselect();
            }
        }
    }

   
    //Strange
    fn handle_normal_mode_key_event_warn(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            if let Some(username) = get_username(
                &self.base_client.username,
                &app.items.items.get(idx).unwrap().text,
                &self.config.members_tag,
            ) {
                app.input = format!("!warn @{} ", username);
                app.input_idx = app.input.width();
                app.input_mode = InputMode::Editing;
                app.items.unselect();
            }
        }
    }
    fn handle_normal_mode_key_event_page_up(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            app.items.state.select(idx.checked_sub(10).or(Some(0)));
        } else {
            app.items.next();
        }
    }

    fn handle_normal_mode_key_event_page_down(&mut self, app: &mut App) {
        if let Some(idx) = app.items.state.selected() {
            let wanted_idx = idx + 10;
            let max_idx = app.items.items.len() - 1;
            let new_idx = std::cmp::min(wanted_idx, max_idx);
            app.items.state.select(Some(new_idx));
        } else {
            app.items.next();
        }
    }

    fn handle_normal_mode_key_event_esc(&mut self, app: &mut App) {
        app.items.unselect();
    }

    fn handle_normal_mode_key_event_shift_u(&mut self, app: &mut App) {
        app.items.state.select(Some(0));
    }

    fn handle_editing_mode_key_event_enter(&mut self, app: &mut App) -> Result<(), ExitSignal> {
        if FIND_RGX.is_match(&app.input) {
            return Ok(());
        }

        let input: String = app.input.drain(..).collect();
        app.input_idx = 0;

        // Iterate over commands and execute associated actions
        for (command, action) in &app.commands.commands {
            // log::error!("command :{} action :{}", command, action);
            let expected_input = format!("!{}", command);
            if input == expected_input {
                // Execute the action by posting a message
                self.post_msg(PostType::Post(action.clone(), None)).unwrap();
                // Return Ok(()) if the action is executed successfully
                return Ok(());
            }
        }

        if input == "/dl" {
            // Delete last message
            self.post_msg(PostType::DeleteLast).unwrap();
        } else if let Some(captures) = DLX_RGX.captures(&input) {
            // Delete the last X messages
            let x: usize = captures.get(1).unwrap().as_str().parse().unwrap();
            for _ in 0..x {
                self.post_msg(PostType::DeleteLast).unwrap();
            }
        } else if input == "/dall" {
            // Delete all messages
            self.post_msg(PostType::DeleteAll).unwrap();
        } else if input == "/cycles" {
            self.color_tx.send(()).unwrap();
        } else if input == "/cycle1" {
            self.start_cycle(true);
        } else if input == "/cycle2" {
            self.start_cycle(false);
        } else if input == "/kall" {
            // Kick all guests
            let username = "s _".to_owned();
            let msg = "".to_owned();
            self.post_msg(PostType::Kick(msg, username)).unwrap();
        } else if input.starts_with("/m ") {
            // Send message to "members" section
            let msg = remove_prefix(&input, "/m ").to_owned();
            let to = Some(SEND_TO_MEMBERS.to_owned());
            self.post_msg(PostType::Post(msg, to)).unwrap();
            app.input = "/m ".to_owned();
            app.input_idx = app.input.width()
        } else if input.starts_with("/a ") {
            // Send message to "admin" section
            let msg = remove_prefix(&input, "/a ").to_owned();
            let to = Some(SEND_TO_ADMINS.to_owned());
            self.post_msg(PostType::Post(msg, to)).unwrap();
            app.input = "/a ".to_owned();
            app.input_idx = app.input.width()
        } else if input.starts_with("/s ") {
            // Send message to "staff" section
            let msg = remove_prefix(&input, "/s ").to_owned();
            let to = Some(SEND_TO_STAFFS.to_owned());
            self.post_msg(PostType::Post(msg, to)).unwrap();
            app.input = "/s ".to_owned();
            app.input_idx = app.input.width()
        } else if let Some(captures) = PM_RGX.captures(&input) {
            // Send PM message
            let username = &captures[1];
            let msg = captures[2].to_owned();
            let to = Some(username.to_owned());
            self.post_msg(PostType::Post(msg, to)).unwrap();
            app.input = format!("/pm {} ", username);
            app.input_idx = app.input.width()
        } else if let Some(captures) = NEW_NICKNAME_RGX.captures(&input) {
            // Change nickname
            let new_nickname = captures[1].to_owned();
            self.post_msg(PostType::NewNickname(new_nickname)).unwrap();
        } else if let Some(captures) = NEW_COLOR_RGX.captures(&input) {
            // Change color
            let new_color = captures[1].to_owned();
            self.post_msg(PostType::NewColor(new_color)).unwrap();
        } else if let Some(captures) = KICK_RGX.captures(&input) {
            // Kick a user
            let username = captures[1].to_owned();
            let msg = captures[2].to_owned();
            self.post_msg(PostType::Kick(msg, username)).unwrap();
        } else if let Some(captures) = IGNORE_RGX.captures(&input) {
            // Ignore a user
            let username = captures[1].to_owned();
            self.post_msg(PostType::Ignore(username)).unwrap();
        } else if let Some(captures) = UNIGNORE_RGX.captures(&input) {
            // Unignore a user
            let username = captures[1].to_owned();
            self.post_msg(PostType::Unignore(username)).unwrap();
        } else if let Some(captures) = UPLOAD_RGX.captures(&input) {
            // Upload a file
            let file_path = captures[1].to_owned();
            let send_to = match captures.get(2) {
                Some(to_match) => match to_match.as_str() {
                    "members" => SEND_TO_MEMBERS,
                    "staffs" => SEND_TO_STAFFS,
                    "admins" => SEND_TO_ADMINS,
                    _ => SEND_TO_ALL,
                },
                None => SEND_TO_ALL,
            }
            .to_owned();
            let msg = match captures.get(3) {
                Some(msg_match) => msg_match.as_str().to_owned(),
                None => "".to_owned(),
            };
            self.post_msg(PostType::Upload(file_path, send_to, msg))
                .unwrap();
        } else if input.starts_with("!warn") {
            // Strange
            let msg: String = input
                .find('@')
                .map(|index| input[index..].to_string())
                .unwrap_or_else(String::new);

            let end_msg = format!(
                "This is your warning - {}, will be kicked next  !rules",
                msg
            );
            // log::error!("The Strange end_msg is :{}", end_msg);
            self.post_msg(PostType::Post(end_msg, None)).unwrap();
        } else {
            if input.starts_with("/") && !input.starts_with("/me ") {
                app.input_idx = input.len();
                app.input = input;
                app.input_mode = InputMode::EditingErr;
            } else {
                // Send normal message
                self.post_msg(PostType::Post(input, None)).unwrap();
            }
        }
        Ok(())
    }

    fn handle_editing_mode_key_event_tab(&mut self, app: &mut App, users: &Arc<Mutex<Users>>) {
        let (p1, p2) = app.input.split_at(app.input_idx);
        if p2 == "" || p2.chars().nth(0) == Some(' ') {
            let mut parts: Vec<&str> = p1.split(" ").collect();
            if let Some(user_prefix) = parts.pop() {
                let mut should_autocomplete = false;
                let mut prefix = "";
                if parts.len() == 1
                    && ((parts[0] == "/kick" || parts[0] == "/k")
                        || parts[0] == "/pm"
                        || parts[0] == "/ignore"
                        || parts[0] == "/unignore")
                {
                    should_autocomplete = true;
                } else if user_prefix.starts_with("@") {
                    should_autocomplete = true;
                    prefix = "@";
                }
                if should_autocomplete {
                    let user_prefix_norm = remove_prefix(user_prefix, prefix);
                    let user_prefix_norm_len = user_prefix_norm.len();
                    if let Some(name) = autocomplete_username(users, user_prefix_norm) {
                        let complete_name = format!("{}{}", prefix, name);
                        parts.push(complete_name.as_str());
                        let p2 = p2.trim_start();
                        if p2 != "" {
                            parts.push(p2);
                        }
                        app.input = parts.join(" ");
                        app.input_idx += name.len() - user_prefix_norm_len;
                    }
                }
            }
        }
    }

    fn handle_editing_mode_key_event_ctrl_c(&mut self, app: &mut App) {
        app.clear_filter();
        app.input = "".to_owned();
        app.input_idx = 0;
        app.input_mode = InputMode::Normal;
    }

    fn handle_editing_mode_key_event_ctrl_a(&mut self, app: &mut App) {
        app.input_idx = 0;
    }

    fn handle_editing_mode_key_event_ctrl_e(&mut self, app: &mut App) {
        app.input_idx = app.input.width();
    }

    fn handle_editing_mode_key_event_ctrl_f(&mut self, app: &mut App) {
        if let Some(idx) = app.input.chars().skip(app.input_idx).position(|c| c == ' ') {
            app.input_idx = std::cmp::min(app.input_idx + idx + 1, app.input.width());
        } else {
            app.input_idx = app.input.width();
        }
    }

    fn handle_editing_mode_key_event_ctrl_b(&mut self, app: &mut App) {
        if let Some(idx) = app.input_idx.checked_sub(2) {
            let tmp = app
                .input
                .chars()
                .take(idx)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
            if let Some(idx) = tmp.chars().position(|c| c == ' ') {
                app.input_idx = std::cmp::max(tmp.width() - idx, 0);
            } else {
                app.input_idx = 0;
            }
        }
    }

    fn handle_editing_mode_key_event_ctrl_v(&mut self, app: &mut App) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Ok(clipboard) = ctx.get_contents() {
            let byte_position = byte_pos(&app.input, app.input_idx).unwrap();
            app.input.insert_str(byte_position, &clipboard);
            app.input_idx += clipboard.width();
        }
    }

    fn handle_editing_mode_key_event_left(&mut self, app: &mut App) {
        if app.input_idx > 0 {
            app.input_idx -= 1;
        }
    }

    fn handle_editing_mode_key_event_right(&mut self, app: &mut App) {
        if app.input_idx < app.input.width() {
            app.input_idx += 1;
        }
    }

    fn handle_editing_mode_key_event_down(&mut self, app: &mut App) {
        app.input_mode = InputMode::Normal;
        app.items.next();
    }

    fn handle_editing_mode_key_event_shift_c(&mut self, app: &mut App, c: char) {
        let byte_position = byte_pos(&app.input, app.input_idx).unwrap();
        app.input.insert(byte_position, c);

        app.input_idx += 1;
        app.update_filter();
    }

    fn handle_editing_mode_key_event_backspace(&mut self, app: &mut App) {
        if app.input_idx > 0 {
            app.input_idx -= 1;
            app.input = remove_at(&app.input, app.input_idx);
            app.update_filter();
        }
    }

    fn handle_editing_mode_key_event_delete(&mut self, app: &mut App) {
        if app.input_idx > 0 && app.input_idx == app.input.width() {
            app.input_idx -= 1;
        }
        app.input = remove_at(&app.input, app.input_idx);
        app.update_filter();
    }

    fn handle_editing_mode_key_event_esc(&mut self, app: &mut App) {
        app.input_mode = InputMode::Normal;
    }

    fn handle_mouse_event(
        &mut self,
        app: &mut App,
        mouse_event: MouseEvent,
    ) -> Result<(), ExitSignal> {
        match mouse_event.kind {
            MouseEventKind::ScrollDown => app.items.next(),
            MouseEventKind::ScrollUp => app.items.previous(),
            _ => {}
        }
        Ok(())
    }
}

// Give a char index, return the byte position
fn byte_pos(v: &str, idx: usize) -> Option<usize> {
    let mut b = 0;
    let mut chars = v.chars();
    for _ in 0..idx {
        if let Some(c) = chars.next() {
            b += c.len_utf8();
        } else {
            return None;
        }
    }
    Some(b)
}

// Remove the character at idx (utf-8 aware)
fn remove_at(v: &str, idx: usize) -> String {
    v.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i == idx {
                return None;
            }
            Some(c)
        })
        .collect::<String>()
}

// Autocomplete any username
fn autocomplete_username(users: &Arc<Mutex<Users>>, prefix: &str) -> Option<String> {
    let users = users.lock().unwrap();
    let all_users = users.all();
    let prefix_lower = prefix.to_lowercase();
    let filtered = all_users
        .iter()
        .find(|(_, name)| name.to_lowercase().starts_with(&prefix_lower));
    Some(filtered?.1.to_owned())
}

fn set_profile_base_info(
    client: &Client,
    full_url: &str,
    params: &mut Vec<(&str, String)>,
) -> anyhow::Result<()> {
    params.extend(vec![("action", "profile".to_owned())]);
    let profile_resp = client.post(full_url).form(&params).send()?;
    let profile_resp_txt = profile_resp.text().unwrap();
    let doc = Document::from(profile_resp_txt.as_str());
    let bold = doc.find(Attr("id", "bold")).next().unwrap();
    let italic = doc.find(Attr("id", "italic")).next().unwrap();
    let small = doc.find(Attr("id", "small")).next().unwrap();
    if bold.attr("checked").is_some() {
        params.push(("bold", "on".to_owned()));
    }
    if italic.attr("checked").is_some() {
        params.push(("italic", "on".to_owned()));
    }
    if small.attr("checked").is_some() {
        params.push(("small", "on".to_owned()));
    }
    let font_select = doc.find(Attr("name", "font")).next().unwrap();
    let font = font_select.find(Name("option")).find_map(|el| {
        if el.attr("selected").is_some() {
            return Some(el.attr("value").unwrap());
        }
        None
    });
    params.push(("font", font.unwrap_or("").to_owned()));
    Ok(())
}

enum RetryErr {
    Retry,
    Exit,
}

fn retry_fn<F>(mut clb: F)
where
    F: FnMut() -> anyhow::Result<RetryErr>,
{
    loop {
        match clb() {
            Ok(RetryErr::Retry) => continue,
            Ok(RetryErr::Exit) => return,
            Err(err) => {
                log::error!("{}", err);
                continue;
            }
        }
    }
}

fn post_msg(
    client: &Client,
    post_type_recv: PostType,
    full_url: &str,
    session: String,
    url: &str,
    last_post_tx: &crossbeam_channel::Sender<()>,
) {
    let mut should_reset_keepalive_timer = false;
    retry_fn(|| -> anyhow::Result<RetryErr> {
        let post_type = post_type_recv.clone();
        let resp_text = client.get(url).send()?.text()?;
        let doc = Document::from(resp_text.as_str());
        let nc = doc
            .find(Attr("name", "nc"))
            .next()
            .context("nc not found")?;
        let nc_value = nc.attr("value").context("nc value not found")?.to_owned();
        let postid = doc
            .find(Attr("name", "postid"))
            .next()
            .context("failed to get postid")?;
        let postid_value = postid
            .attr("value")
            .context("failed to get postid value")?
            .to_owned();
        let mut params: Vec<(&str, String)> = vec![
            ("lang", LANG.to_owned()),
            ("nc", nc_value.to_owned()),
            ("session", session.clone()),
        ];

        if let PostType::Clean(date, text) = post_type {
            if let Err(e) = delete_message(&client, full_url, &mut params, date, text) {
                log::error!("failed to delete message: {:?}", e);
                return Ok(RetryErr::Retry);
            }
            return Ok(RetryErr::Exit);
        }

        let mut req = client.post(full_url);
        let mut form: Option<multipart::Form> = None;

        match post_type {
            PostType::Post(msg, send_to) => {
                should_reset_keepalive_timer = true;
                params.extend(vec![
                    ("action", "post".to_owned()),
                    ("postid", postid_value.to_owned()),
                    ("multi", "on".to_owned()),
                    ("message", msg),
                    ("sendto", send_to.unwrap_or(SEND_TO_ALL.to_owned())),
                ]);
            }
            PostType::NewNickname(new_nickname) => {
                set_profile_base_info(client, full_url, &mut params)?;
                params.extend(vec![
                    ("do", "save".to_owned()),
                    ("timestamps", "on".to_owned()),
                    ("newnickname", new_nickname),
                ]);
            }
            PostType::NewColor(new_color) => {
                set_profile_base_info(client, full_url, &mut params)?;
                params.extend(vec![
                    ("do", "save".to_owned()),
                    ("timestamps", "on".to_owned()),
                    ("colour", new_color),
                ]);
            }
            PostType::Ignore(username) => {
                set_profile_base_info(client, full_url, &mut params)?;
                params.extend(vec![
                    ("do", "save".to_owned()),
                    ("timestamps", "on".to_owned()),
                    ("ignore", username),
                ]);
            }
            PostType::Unignore(username) => {
                set_profile_base_info(client, full_url, &mut params)?;
                params.extend(vec![
                    ("do", "save".to_owned()),
                    ("timestamps", "on".to_owned()),
                    ("unignore", username),
                ]);
            }
            PostType::Profile(new_color, new_nickname) => {
                set_profile_base_info(client, full_url, &mut params)?;
                params.extend(vec![
                    ("do", "save".to_owned()),
                    ("timestamps", "on".to_owned()),
                    ("colour", new_color),
                    ("newnickname", new_nickname),
                ]);
            }
            PostType::Kick(msg, send_to) => {
                params.extend(vec![
                    ("action", "post".to_owned()),
                    ("postid", postid_value.to_owned()),
                    ("message", msg),
                    ("sendto", send_to),
                    ("kick", "kick".to_owned()),
                    ("what", "purge".to_owned()),
                ]);
            }
            PostType::DeleteLast | PostType::DeleteAll => {
                params.extend(vec![("action", "delete".to_owned())]);
                if let PostType::DeleteAll = post_type {
                    params.extend(vec![
                        ("sendto", SEND_TO_ALL.to_owned()),
                        ("confirm", "yes".to_owned()),
                        ("what", "all".to_owned()),
                    ]);
                } else {
                    params.extend(vec![("sendto", "".to_owned()), ("what", "last".to_owned())]);
                }
            }
            PostType::Upload(file_path, send_to, msg) => {
                form = Some(
                    match multipart::Form::new()
                        .text("lang", LANG.to_owned())
                        .text("nc", nc_value.to_owned())
                        .text("session", session.clone())
                        .text("action", "post".to_owned())
                        .text("postid", postid_value.to_owned())
                        .text("message", msg)
                        .text("sendto", send_to.to_owned())
                        .text("what", "purge".to_owned())
                        .file("file", file_path)
                    {
                        Ok(f) => f,
                        Err(e) => {
                            log::error!("{:?}", e);
                            return Ok(RetryErr::Exit);
                        }
                    },
                );
            }
            PostType::Clean(_, _) => {}
        }

        if let Some(form_content) = form {
            req = req.multipart(form_content);
        } else {
            req = req.form(&params);
        }
        if let Err(err) = req.send() {
            log::error!("{:?}", err.to_string());
            if err.is_timeout() {
                return Ok(RetryErr::Retry);
            }
        }
        return Ok(RetryErr::Exit);
    });
    if should_reset_keepalive_timer {
        last_post_tx.send(()).unwrap();
    }
}

fn parse_date(date: &str, datetime_fmt: &str) -> NaiveDateTime {
    let now = Utc::now();
    let date_fmt = format!("%Y-{}", datetime_fmt);
    NaiveDateTime::parse_from_str(
        format!("{}-{}", now.year(), date).as_str(),
        date_fmt.as_str(),
    )
    .unwrap()
}

fn get_msgs(
    client: &Client,
    base_url: &str,
    page_php: &str,
    session: &str,
    username: &str,
    users: &Arc<Mutex<Users>>,
    sig: &Arc<Mutex<Sig>>,
    messages_updated_tx: &crossbeam_channel::Sender<()>,
    members_tag: &str,
    datetime_fmt: &str,
    tx: &crossbeam_channel::Sender<PostType>,
    messages: &Arc<Mutex<Vec<Message>>>,
    should_notify: &mut bool,
) -> anyhow::Result<()> {
    let url = format!(
        "{}/{}?action=view&session={}&lang={}",
        base_url, page_php, session, LANG
    );
    let resp_text = client.get(url).send()?.text()?;
    let resp_text = resp_text.replace("<br>", "\n");
    let doc = Document::from(resp_text.as_str());
    let new_messages = match extract_messages(&doc) {
        Ok(messages) => messages,
        Err(_) => {
            // Gagal mendapatkan pesan, mungkin perlu login ulang
            sig.lock().unwrap().signal(&ExitSignal::NeedLogin);
            return Ok(());
        }
    };
    {
        let messages = messages.lock().unwrap();
        process_new_messages(&new_messages, &messages, datetime_fmt, members_tag, username, should_notify, tx, users);
        // Membangun vektor pesan. Menandai pesan yang dihapus.
        update_messages(new_messages, messages, datetime_fmt);
        // Memberi tahu bahwa pesan baru telah tiba.
        // Ini memastikan bahwa kita menggambar ulang pesan di layar segera.
        // Jika tidak, layar tidak akan digambar ulang sampai ada kejadian keyboard.
        messages_updated_tx.send(()).map_err(|_| anyhow::anyhow!("Gagal mengirim sinyal pembaruan pesan"))?;
    }
    {
        let mut users = users.lock().unwrap();
        ban_imposters(tx, username, &users);
        *users = extract_users(&doc);
    }
    Ok(())
}
fn process_new_messages(
    new_messages: &[Message],
    messages: &MutexGuard<Vec<Message>>,
    datetime_fmt: &str,
    members_tag: &str,
    username: &str,
    should_notify: &mut bool,
    tx: &crossbeam_channel::Sender<PostType>,
    users: &Arc<Mutex<Users>>,
) {
    // Inisialisasi struktur data untuk menyimpan nama-nama pengguna
    lazy_static! {
        static ref KNOWN_USERS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    }

    // Fungsi untuk memeriksa apakah pengguna baru
    fn is_new_user(username: &str) -> bool {
        let mut known_users = KNOWN_USERS.lock().unwrap();
        if known_users.contains(username) {
            false
        } else {
            known_users.insert(username.to_string());
            true
        }
    }

    if let Some(last_known_msg) = messages.first() {
        let last_known_msg_parsed_dt = parse_date(&last_known_msg.date, datetime_fmt);
        let filtered = new_messages.iter().filter(|new_msg| {
            parse_date(&new_msg.date, datetime_fmt) > last_known_msg_parsed_dt
                || (new_msg.date == last_known_msg.date && last_known_msg.text != new_msg.text)
        });

        for new_msg in filtered {
            if new_msg.typ == MessageType::SysMsg {
                let system_msg = new_msg.text.text();
                if system_msg.contains("entered the chat.") {
                    if let Some(entered_username) = system_msg.split_whitespace().nth(1) {
                        if is_new_user(entered_username) {
                            let welcome_msg = format!(
                                "[color=#00FF00]Welcome back @{} !![/color]",
                                entered_username
                            );
                            tx.send(PostType::Post(welcome_msg, Some(SEND_TO_MEMBERS.to_owned()))).unwrap();
                        }
                    }
                }
            }

            if let Some((from, to_opt, msg)) = get_message(&new_msg.text, members_tag) {
                *should_notify |= msg.contains(&format!("@{}", username)) 
                    || (to_opt.as_ref().map_or(false, |to| to == username) && msg != "!up");

                unsafe {
                    if BOT_ACTIVE {
                        let users_lock = users.lock().unwrap();
                        dantca_imps_proses(&from, &msg, tx, &users_lock);
                    }
                }

                let users_lock = users.lock().unwrap();
                if from == XPLDAN || users_lock.members.iter().any(|(_color, member)| member.to_lowercase() == from.to_lowercase()) {
                    match msg.as_str() {
                        "dantcaoff!" => toggle_bot_active(false, tx, &from),
                        "dantcago!" => toggle_bot_active(true, tx, &from),
                        "statusdan!" => check_bot_status(tx, &from),
                        "dantcahelp!" => dantca_help(tx, &from),
                        _ => {}
                    }
                }
                if msg.to_lowercase().contains("reportdan!") {
                    report_dantca(tx, &from);
                }
            }
        }
    }
}

fn report_dantca(tx: &crossbeam_channel::Sender<PostType>, from: &str) {
    if from != XPLDAN {
        let report_message = format!("Hallo @{}, to send PM to Dantca bot you can click the '-All chatters-' box above the chat, and select user @XplDan for contact github: @0srd4n, proton: Xpldan@proton.me. or you can donation to my BTC address: [comming soon] ", from);
        tx.send(PostType::Post(report_message, Some(from.to_owned()))).unwrap();
    }
}

fn dantca_help(tx: &crossbeam_channel::Sender<PostType>, from: &str) {
        let help_message = format!("
    Hallo @{}, there is guide for Dantca bot
    dantcago-! = Active Dantca Bot
    dantcaoff-! = Deactive Dantca Bot
    statusdan-! = Check Dantca Bot Status
    dantcahelp-! = Dantca Bot Help
    reportdan-! = for report to Dantca bot
    without ( - )", from);
    tx.send(PostType::Post(help_message, Some(SEND_TO_MEMBERS.to_owned())
    )).unwrap();

}

fn toggle_bot_active(active: bool, tx: &crossbeam_channel::Sender<PostType>, from: &str) {
    unsafe {
        BOT_ACTIVE = active;
    }
    let status_message = if active {
        format!("[color=#ffffff]>[] -- Dantca Actived By @{} -- []<[/color] ", from)
    } else {
        format!("[color=#ffffff]>[] -- Dantca Deactived By @{} -- []<[/color]", from)
    };
    tx.send(PostType::Post(status_message, None)).unwrap();
}

fn check_bot_status(tx: &crossbeam_channel::Sender<PostType>, from: &str) {
    let status = unsafe { BOT_ACTIVE };
    let status_message = if status {
        "> - Dantca Still Running - <"
    } else {
        "> - Dantca Not Running - <"
    };
    let messtats = format!(" [color=#ffffff] {} == [/color] [ @{} ]", status_message, from);
    tx.send(PostType::Post(messtats, Some(SEND_TO_ALL.to_owned()))).unwrap();
}

fn dantca_imps_proses(from: &str, msg: &str, tx: &crossbeam_channel::Sender<PostType>, users: &Users) {

    let msg_lower = msg.to_lowercase();
    let from_lower = from.to_lowercase();
    
    if let Some((_color, _username)) = users.guests.iter().find(|(_color, name)| name.to_lowercase() == from_lower) {
        let username_to_kick = from_lower.clone();
        let (triggered, kicked, warns) = check_message_content(&msg_lower);
        
        let mut warned_users = WARNED_USERS.lock().unwrap();
        let count = warned_users.entry(from_lower.clone()).or_insert(0);
        
        if triggered {
            *count += 1;
            tx.send(PostType::Post(format!(">>> Dantca :  Hallo @{}, ->  [color=#ffffff]you have warns : [/color] [color=#00FF00]| {}/2 |[/color] -> Your Warnings :  {} [BANNED TOPIC]-< [LAST WARNS] <<<", username_to_kick, *count, warns), Some(SEND_TO_ALL.to_owned()))).unwrap();
        }
        
        if *count >= 2 {
            tx.send(PostType::Kick(format!(">>> Dantca : Hallo  @{}, You have been warned multiple waarns | = {} = |times and are now being kicked. <<< ", username_to_kick, *count), username_to_kick.clone())).unwrap();
        }
        
        if kicked {
            tx.send(PostType::Post(format!(">>> Dantca : Hallo ,@{} -> your warnings: {} [BANNED TOPIC]-< <<<", username_to_kick, warns), Some(SEND_TO_ALL.to_owned()))).unwrap();
        }
        
        // Menggunakan match untuk menangani berbagai kasus pesan
        match msg_lower.as_str() {
            m if m.contains("chat?") || m.contains("what about chat?") => 
                tx.send(PostType::Post(format!("Halo, @{} !about", from), Some(SEND_TO_ALL.to_owned()))).unwrap(),
            m if m.contains("bhcli") => 
                tx.send(PostType::Post(format!("hallo, @{} -> {}", from, BHCLI_BLOG_URL), None)).unwrap(),
            m if m.contains("i new here") || m.contains("im new here") => 
                tx.send(PostType::Post(format!("[color=#ffffff]Hallo,@{} Welcome the Black Hat Chat You can try !-help and !-newmembers to more informations.-< [/color]", from), None)).unwrap(),
            m if m.contains("learn programming") || m.contains("learn python") || m.contains("how can i start hacking") => 
                tx.send(PostType::Post(format!("Halo @{} you can try !-learn!-ctf ~Dantca bot", from), Some(SEND_TO_ALL.to_owned()))).unwrap(),
            m if m.contains("!-help") || m.contains("how to send @0?") || m.contains("list of commands") => 
                tx.send(PostType::Post(format!("[color=#ffffff]Hello @{} This command can be sent to user @0 so as not to pollute the chat. To send a message to a user, click the '-All chatters-' box above the chat, and select user @0.[/color]", from), Some(SEND_TO_ALL.to_owned()))).unwrap(),
            m if m.contains("red room") || m.contains("redroom") || m.contains("link red room") || m.contains("link redroom") => 
                tx.send(PostType::Post(format!("Halo @{},!redroom", from), Some(SEND_TO_ALL.to_owned()))).unwrap(),
            m if m.contains("pedo") || m.contains("hacking services") || m.contains("child porn") || m.contains("fuck all") || m.contains("fuck off") || m.contains("gore video") || m.contains("link gore") || m.contains("+62") || m.contains("nigger") || m.contains("stupid all") || m.contains("nigga") || m.contains("masturbate") || m.contains(" CP ") || m.contains("link cp") || m.contains("horny")  || m.contains("porn video") => 
                tx.send(PostType::Kick(format!("!! dont break the rules you can try !-help or !-rules blacklist word '{}'  ~bot XplDan", m), username_to_kick)).unwrap(),
            m if (m.contains("links") || m.contains("dark web link") || m.contains("link dark web")) && (m.contains("where ") || m.contains("want ") || m.contains("lookin") || m.contains("know ") || m.contains("have ") || m.contains("need ")) => 
                tx.send(PostType::Post(format!("hallo @{} you can try !-links and send to @0", from), None)).unwrap(),
            _ => {}
        }
    }
}

fn check_message_content(msg: &str) -> (bool, bool, &str) {
    let msgcopy = msg.to_lowercase();
    let mut triggered = false;
    let mut kicked = false;
    let mut warns = "";

    if msgcopy.contains("betting") &&
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Betting is frowned upon here";
        triggered = true;
    }
    if (msgcopy.contains("buy") || msgcopy.contains("sell ")) && ((msgcopy.contains("credit") && msgcopy.contains("card ")) || msgcopy.contains("cc ")) &&
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Credit Carding is a";
        triggered = true;
    }
    if (msgcopy.contains("buy ") || msgcopy.contains("sell ")) && msgcopy.contains("gun") && 
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Munitions and talk thereof is a ";
        triggered = true;
    }
    if msgcopy.contains("porn") && 
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin ") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Porn is a ";
        triggered = true;
    }
    
    if msgcopy.contains("torture") && 
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Torture is a ";
        triggered = true;
    }        
    
    if msgcopy.contains("cock ") && (!msgcopy.contains("cock.li")) &&
        ( 
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        )
    {
        warns = "Poor taste";
        triggered = true;
    }
    if msgcopy.contains("hack") && (msgcopy.contains(" fb ") || msgcopy.contains(" insta ") || msgcopy.contains(" twitter ") || msgcopy.contains(" facebook ") || msgcopy.contains(" instagram ")) &&
        ( 
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Social Media Hacking is bad form ";
        triggered = true;
    }             
    if msgcopy.contains("ableonion ") {
        warns = "Able Onion is a ";
        triggered = true;
    }      
    if msgcopy.contains("cp ") && 
        ( 
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "CP is a ";
        triggered = true;
        kicked = true;
    }
    if msgcopy.contains("rape ") && 
        ( 
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Rape is a ";
        triggered = true;
    }
    if ( msgcopy.contains("loli") || msgcopy.contains("child") || msgcopy.contains("minor") ) && 
        ( 
            msgcopy.contains("content") || 
            msgcopy.contains("porn")
        ) &&
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        )
    {
        warns = "CSAM is a ";
        triggered = true;
    }
    if msgcopy.contains("sex") && msgcopy.contains("cam") {
        warns = "Sex Cams are poor taste";
        triggered = true;
    }                    
    if ( msgcopy.contains("buy") || msgcopy.contains("sell") ) && ( msgcopy.contains("human") ) {
        warns = "Human sales is poor taste";
        triggered = true;
    }                                            
    if msgcopy.contains("market") && ( msgcopy.contains("black") || msgcopy.contains("under") ) {
        warns = "Markets are bad, 98% are scams";
        triggered = true;
    }


    if msgcopy.contains("p5hwh3fxfb4x22rpmgq32c3xps6g6k6rvmualzj4gwvxs5ovjhbd4fyd.onion") {
        warns = "We don't like your link.";
        kicked = true;
    } 

    if ( msgcopy.contains("hitman") || msgcopy.contains("hitmen") ) && 
        (     
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Hitmen have nothing to do with us LOL";
        kicked = true;
    }        
    if msgcopy.contains("nogg") || msgcopy.contains("niqq") || msgcopy.contains("nigg") || msgcopy.contains("nigga") || msgcopy.contains("nigge") || msgcopy.contains("niggo") || msgcopy.contains("niggi") || msgcopy.contains("niggu")  {
        warns = "Offensive terms are bad form. SORRY..";
        kicked = true;
    }
    if msgcopy.contains("indian") &&
        ( 
            msgcopy.contains("ni") ||
            msgcopy.contains("shit") ||
            msgcopy.contains("fuck") 
        ) 
    {
        warns = "Racial Insults won't be tolerated.";
        triggered = true;
        kicked = true;
    }             
    if msgcopy.contains("bomb") && 
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")            
        ) 
    {
        warns = "Munitions is a ";
        kicked = true;
    }
    if msgcopy.contains("database") || msgcopy.contains("db") && msgcopy.contains("dump") &&
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Databases are not us. Be gone...";
        kicked = true;
    }
    if msgcopy.contains("paypal") && msgcopy.contains("transfer") && 
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Paypal - not here PAL! Be gone....";
        kicked = true;
    }
    if msgcopy.to_lowercase().contains("cc ") && 
        (
            msgcopy.contains("make") || 
            msgcopy.contains("dump") ||
            msgcopy.contains("where") || 
            msgcopy.contains("want") || 
            msgcopy.contains("lookin") || 
            msgcopy.contains("know") ||         		
            msgcopy.contains("have") ||
            msgcopy.contains("sell") ||
            msgcopy.contains("share") ||
            msgcopy.contains("buy")
        ) 
    {
        warns = "Carding is a";
        kicked = true;
    }
    if msgcopy.contains("tabularis") && 
        (
            msgcopy.contains("where ")
            || msgcopy.contains("want ")
            || msgcopy.contains("lookin") 
            || msgcopy.contains("know ")
            || msgcopy.contains("have ")
            || msgcopy.contains("need ")
        ) 
    {
        warns = "Tabularis - not here! Be gone... BYE BYE...";
        kicked = true;
    }

    (triggered, kicked, warns)
}
fn ban_imposters(tx: &crossbeam_channel::Sender<PostType>, account_username: &str, users: &Users) {
    if BAN_IMPOSTERS {

    // Hanya jalankan jika tidak ada admin atau staff (kecuali XPLDAN)
    if !users.admin.is_empty() || (!users.staff.is_empty() && account_username != XPLDAN) {
        return;
    }

    let banned_patterns = [
        (Regex::new(r"(?i)n[o0]tr[1il][vy]").unwrap(), "n0tr1v"),
        (Regex::new(r"(?i)h[i1]t[l1]er").unwrap(), "hitler"),
        (Regex::new(r"(?i)h[i1]m+l[e3]r").unwrap(), "himmler"),
        (Regex::new(r"(?i)m[e3]ng[e3]l[e3]").unwrap(), "mengele"),
        (Regex::new(r"(?i)g[o0][e3]b+[e3]ls").unwrap(), "goebbels"),
        (Regex::new(r"(?i)h[e3]ydr[i1]ch").unwrap(), "heydrich"),
        (Regex::new(r"(?i)gl[o0]b[o0]cn[i1l]k").unwrap(), "globocnik"),
        (Regex::new(r"(?i)d[i1]rl[e3]wang[e3]r").unwrap(), "dirlewanger"),
        (Regex::new(r"(?i)j[e3]ck[e3]ln").unwrap(), "jeckeln"),
        (Regex::new(r"(?i)kram[e3]r").unwrap(), "kramer"),
        (Regex::new(r"(?i)bl[o0]b[e3]l").unwrap(), "blobel"),
        (Regex::new(r"(?i)stangl").unwrap(), "stangl"),
        (Regex::new(r"(?i)\b(pedo|cp|danbyt|bigdick|bitch|kill|killer|dick|trolls|child\s*porn|hamas|pussy|cum|pedofile|fucked|lolita\s*slaves|fuck\s*all|fucking|bomb|fuckings)\b").unwrap(), "general blacklist"),
    ];

    let xpldan_patterns = Regex::new(r"(?i)\b(fuck|xpldan|nigg[iuaoe]|nig[iuao]|niqq|chink|wank|shit|cunt|bitch|booty|hooker|milf|rapist|balls|sex|cocaine|heroine|weed|drug|card|fisting|jerk|p3do|pedo|cplove|perv|gangbang|porn|dick|penis|puzzy|pussy|boceta|anal|cum|market|sell|fraud|DN37R34P3R|atomwaffen|altright)\b").unwrap();

    for (_color, username) in &users.guests {
        let lower_name = username.to_lowercase();

        // Cek nama member
        if users.members.iter().any(|(_, member)| lower_name.contains(&member.to_lowercase())) {
            let msg = format!("Username members BHC '{}' is not allowed.", username);
            tx.send(PostType::Kick(msg, username.to_owned())).unwrap();
            continue;
        }

        // Cek pola yang dilarang
        for (pattern, name) in &banned_patterns {
            if pattern.is_match(&lower_name) {
                let msg = format!("Do not use names on the blacklist '{}' ({}).", lower_name, name);
                tx.send(PostType::Kick(msg, username.to_owned())).unwrap();
                break;
            }
        }

        // Cek pola tambahan untuk XPLDAN
        if account_username == XPLDAN && xpldan_patterns.is_match(&lower_name) {
            let msg = format!("Do not use names that are on the blacklist '{}' bot ~Dantca", lower_name);
            tx.send(PostType::Kick(msg, username.to_owned())).unwrap();
        }
    }
}
}

fn update_messages(
    new_messages: Vec<Message>,
    mut messages: MutexGuard<Vec<Message>>,
    datetime_fmt: &str,
) {
    let mut old_msg_ptr = 0;
    for new_msg in new_messages.into_iter() {
        loop {
            if let Some(old_msg) = messages.get_mut(old_msg_ptr) {
                let new_parsed_dt = parse_date(&new_msg.date, datetime_fmt);
                let parsed_dt = parse_date(&old_msg.date, datetime_fmt);
                if new_parsed_dt < parsed_dt {
                    old_msg.deleted = true;
                    old_msg_ptr += 1;
                    continue;
                }
                if new_parsed_dt == parsed_dt {
                    if old_msg.text != new_msg.text {
                        let mut found = false;
                        let mut x = 0;
                        loop {
                            x += 1;
                            if let Some(old_msg) = messages.get(old_msg_ptr + x) {
                                let parsed_dt = parse_date(&old_msg.date, datetime_fmt);
                                if new_parsed_dt == parsed_dt {
                                    if old_msg.text == new_msg.text {
                                        found = true;
                                        break;
                                    }
                                    continue;
                                }
                            }
                            break;
                        }
                        if !found {
                            messages.insert(old_msg_ptr, new_msg);
                            old_msg_ptr += 1;
                        }
                    }
                    old_msg_ptr += 1;
                    break;
                }
            }
            messages.insert(old_msg_ptr, new_msg);
            old_msg_ptr += 1;
            break;
        }
    }
    messages.truncate(1000);
}

fn delete_message(
    client: &Client,
    full_url: &str,
    params: &mut Vec<(&str, String)>,
    date: String,
    text: String,
) -> anyhow::Result<()> {
    params.extend(vec![
        ("action", "admin".to_owned()),
        ("do", "clean".to_owned()),
        ("what", "choose".to_owned()),
    ]);
    let clean_resp_txt = client.post(full_url).form(&params).send()?.text()?;
    let doc = Document::from(clean_resp_txt.as_str());
    let nc = doc
        .find(Attr("name", "nc"))
        .next()
        .context("nc not found")?;
    let nc_value = nc.attr("value").context("nc value not found")?.to_owned();
    let msgs = extract_messages(&doc)?;
    if let Some(msg) = msgs
        .iter()
        .find(|m| m.date == date && m.text.text() == text)
    {
        let msg_id = msg.id.context("msg id not found")?;
        params.extend(vec![
            ("nc", nc_value.to_owned()),
            ("what", "selected".to_owned()),
            ("mid[]", format!("{}", msg_id)),
        ]);
        client.post(full_url).form(&params).send()?;
    }
    Ok(())
}

impl ChatClient {
    fn new(params: Params) -> Self {
        // println!("session[2026] : {:?}",params.session);
        let mut c = new_default_le_chat_php_client(params.clone());
        c.config.url = params.url.unwrap_or(
            "http://blkhatjxlrvc5aevqzz5t6kxldayog6jlx5h7glnu44euzongl4fh5ad.onion/index.php"
                .to_owned(),
        );
        c.config.page_php = params.page_php.unwrap_or("chat.php".to_owned());
        c.config.datetime_fmt = params.datetime_fmt.unwrap_or("%m-%d %H:%M:%S".to_owned());
        c.config.members_tag = params.members_tag.unwrap_or("[M] ".to_owned());
        c.config.keepalive_send_to = params.keepalive_send_to.unwrap_or("0".to_owned());
        // c.session = params.session;
        Self {
            le_chat_php_client: c,
        }
    }

    fn run_forever(&mut self) {
        self.le_chat_php_client.run_forever();
    }
}

fn new_default_le_chat_php_client(params: Params) -> LeChatPHPClient {
    let (color_tx, color_rx) = crossbeam_channel::unbounded();
    let (tx, rx) = crossbeam_channel::unbounded();
    let session = params.session.clone();
    // println!("session[2050] : {:?}",params.session);
    LeChatPHPClient {
        base_client: BaseClient {
            username: params.username,
            password: params.password,
        },
        max_login_retry: params.max_login_retry,
        guest_color: params.guest_color,
        // session: params.session,
        session,
        last_key_event: None,
        client: params.client,
        manual_captcha: params.manual_captcha,
        sxiv: params.sxiv,
        refresh_rate: params.refresh_rate,
        config: LeChatPHPConfig::new_black_hat_chat_config(),
        is_muted: Arc::new(Mutex::new(false)),
        show_sys: false,
        display_guest_view: false,
        display_member_view: false,
        display_hidden_msgs: false,
        tx,
        rx: Arc::new(Mutex::new(rx)),
        color_tx,
        color_rx: Arc::new(Mutex::new(color_rx)),
    }
}

struct ChatClient {
    le_chat_php_client: LeChatPHPClient,
}

#[derive(Debug, Clone)]
struct Params {
    url: Option<String>,
    page_php: Option<String>,
    datetime_fmt: Option<String>,
    members_tag: Option<String>,
    username: String,
    password: String,
    guest_color: String,
    client: Client,
    manual_captcha: bool,
    sxiv: bool,
    refresh_rate: u64,
    max_login_retry: isize,
    keepalive_send_to: Option<String>,
    session: Option<String>,
}

#[derive(Clone)]
enum ExitSignal {
    Terminate,
    NeedLogin,
}
struct Sig {
    tx: crossbeam_channel::Sender<ExitSignal>,
    rx: crossbeam_channel::Receiver<ExitSignal>,
    nb_rx: usize,
}

impl Sig {
    fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let nb_rx = 0;
        Self { tx, rx, nb_rx }
    }

    fn clone(&mut self) -> crossbeam_channel::Receiver<ExitSignal> {
        self.nb_rx += 1;
        self.rx.clone()
    }

    fn signal(&self, signal: &ExitSignal) {
        for _ in 0..self.nb_rx {
            self.tx.send(signal.clone()).unwrap();
        }
    }
}

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn get_guest_color(wanted: Option<String>) -> String {
    match wanted.as_deref() {
        Some("beige") => "F5F5DC",
        Some("blue-violet") => "8A2BE2",
        Some("brown") => "A52A2A",
        Some("cyan") => "00FFFF",
        Some("sky-blue") => "00BFFF",
        Some("gold") => "FFD700",
        Some("gray") => "808080",
        Some("green") => "008000",
        Some("hot-pink") => "FF69B4",
        Some("light-blue") => "ADD8E6",
        Some("light-green") => "90EE90",
        Some("lime-green") => "32CD32",
        Some("magenta") => "FF00FF",
        Some("olive") => "808000",
        Some("orange") => "FFA500",
        Some("orange-red") => "FF4500",
        Some("red") => "FF0000",
        Some("royal-blue") => "4169E1",
        Some("see-green") => "2E8B57",
        Some("sienna") => "A0522D",
        Some("silver") => "C0C0C0",
        Some("tan") => "D2B48C",
        Some("teal") => "008080",
        Some("violet") => "EE82EE",
        Some("white") => "FFFFFF",
        Some("yellow") => "FFFF00",
        Some("yellow-green") => "9ACD32",
        Some(other) => COLOR1_RGX
            .captures(other)
            .map_or("", |captures| captures.get(1).map_or("", |m| m.as_str())),
        None => "",
    }
    .to_owned()
}

fn get_tor_client(socks_proxy_url: &str, no_proxy: bool) -> Client {
    let ua = "Mozilla/5.0 (Windows NT 10.0; rv:102.0) Gecko/20100101 Firefox/102.0";
    let mut builder = reqwest::blocking::ClientBuilder::new()
        .redirect(Policy::none())
        .cookie_store(true)
        .user_agent(ua);
    if !no_proxy {
        let proxy = reqwest::Proxy::all(socks_proxy_url).unwrap();
        builder = builder.proxy(proxy);
    }
    builder.build().unwrap()
}

fn ask_username(username: Option<String>) -> String {
    username.unwrap_or_else(|| {
        print!("username: ");
        let mut username_input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut username_input).unwrap();
        trim_newline(&mut username_input);
        username_input
    })
}

fn ask_password(password: Option<String>) -> String {
    password.unwrap_or_else(|| rpassword::prompt_password("Password: ").unwrap())
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DkfNotifierResp {
    #[serde(rename = "NewMessageSound")]
    pub new_message_sound: bool,
    #[serde(rename = "TaggedSound")]
    pub tagged_sound: bool,
    #[serde(rename = "PmSound")]
    pub pm_sound: bool,
    #[serde(rename = "InboxCount")]
    pub inbox_count: i64,
    #[serde(rename = "LastMessageCreatedAt")]
    pub last_message_created_at: String,
}

fn start_dkf_notifier(client: &Client, dkf_api_key: &str) {
    let client = client.clone();
    let dkf_api_key = dkf_api_key.to_owned();
    let mut last_known_date = Utc::now();
    thread::spawn(move || loop {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let source = Decoder::new_mp3(Cursor::new(SOUND1)).unwrap();

        let params: Vec<(&str, String)> = vec![(
            "last_known_date",
            last_known_date.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        )];
        let right_url = format!("{}/api/v1/chat/1/notifier", DKF_URL);
        if let Ok(resp) = client
            .post(right_url)
            .form(&params)
            .header("DKF_API_KEY", &dkf_api_key)
            .send()
        {
            if let Ok(txt) = resp.text() {
                if let Ok(v) = serde_json::from_str::<DkfNotifierResp>(&txt) {
                    if v.pm_sound || v.tagged_sound {
                        stream_handle.play_raw(source.convert_samples()).unwrap();
                    }
                    last_known_date = DateTime::parse_from_rfc3339(&v.last_message_created_at)
                        .unwrap()
                        .with_timezone(&Utc);
                }
            }
        }
        thread::sleep(Duration::from_secs(5));
    });
}

// Start thread that looks for new emails on DNMX every minutes.
fn start_dnmx_mail_notifier(client: &Client, username: &str, password: &str) {
    let params: Vec<(&str, &str)> = vec![("login_username", username), ("secretkey", password)];
    let login_url = format!("{}/src/redirect.php", DNMX_URL);
    client.post(login_url).form(&params).send().unwrap();

    let client_clone = client.clone();
    thread::spawn(move || loop {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let source = Decoder::new_mp3(Cursor::new(SOUND1)).unwrap();

        let right_url = format!("{}/src/right_main.php", DNMX_URL);
        if let Ok(resp) = client_clone.get(right_url).send() {
            let mut nb_mails = 0;
            let doc = Document::from(resp.text().unwrap().as_str());
            if let Some(table) = doc.find(Name("table")).nth(7) {
                table.find(Name("tr")).skip(1).for_each(|n| {
                    if let Some(td) = n.find(Name("td")).nth(2) {
                        if td.find(Name("b")).nth(0).is_some() {
                            nb_mails += 1;
                        }
                    }
                });
            }
            if nb_mails > 0 {
                log::error!("{} new mails", nb_mails);
                stream_handle.play_raw(source.convert_samples()).unwrap();
            }
        }
        thread::sleep(Duration::from_secs(60));
    });
}

//Strange
#[derive(Debug, Deserialize)]
struct Commands {
    commands: HashMap<String, String>,
}

impl Default for Commands {
    fn default() -> Self {
        Commands {
            commands: HashMap::new(), // Initialize commands with empty HashMap
        }
    }
}

// Strange
// Function to read the configuration file and parse it
fn read_commands_file(file_path: &str) -> Result<Commands, Box<dyn std::error::Error>> {
    // Read the contents of the file
    let commands_content = std::fs::read_to_string(file_path)?;
    // log::error!("Read file contents: {}", commands_content);
    // Deserialize the contents into a Commands struct
    let commands: Commands = toml::from_str(&commands_content)?;
    // log::error!(
    //     "Deserialized file contents into Commands struct: {:?}",
    //     commands
    // );

    Ok(commands)
}

fn main() -> anyhow::Result<()> {
    let mut opts: Opts = Opts::parse();
    // println!("Parsed Session: {:?}", opts.session);


    // Configs file
    if let Ok(config_path) = confy::get_configuration_file_path("bhcli", None) {
        println!("Config path: {:?}", config_path);
    }
    if let Ok(cfg) = confy::load::<MyConfig>("bhcli", None) {
        if opts.dkf_api_key.is_none() {
            opts.dkf_api_key = cfg.dkf_api_key;
        }
        if let Some(default_profile) = cfg.profiles.get(&opts.profile) {
            if opts.username.is_none() {
                opts.username = Some(default_profile.username.clone());
                opts.password = Some(default_profile.password.clone());
            }
        }
    }

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} {l} {t} - {m}{n}")))
        .build("bhcli.log")?;

    let config = log4rs::config::Config::builder()
        .appender(log4rs::config::Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            log4rs::config::Root::builder()
                .appender("logfile")
                .build(LevelFilter::Error),
        )?;

    log4rs::init_config(config)?;

    let client = get_tor_client(&opts.socks_proxy_url, opts.no_proxy);

    // If dnmx username is set, start mail notifier thread
    if let Some(dnmx_username) = opts.dnmx_username {
        start_dnmx_mail_notifier(&client, &dnmx_username, &opts.dnmx_password.unwrap())
    }

    if let Some(dkf_api_key) = &opts.dkf_api_key {
        start_dkf_notifier(&client, dkf_api_key);
    }

    let guest_color = get_guest_color(opts.guest_color);
    let username = ask_username(opts.username);
    let password = ask_password(opts.password);

    let params = Params {
        url: opts.url,
        page_php: opts.page_php,
        datetime_fmt: opts.datetime_fmt,
        members_tag: opts.members_tag,
        username,
        password,
        guest_color,
        client: client.clone(),
        manual_captcha: opts.manual_captcha,
        sxiv: opts.sxiv,
        refresh_rate: opts.refresh_rate,
        max_login_retry: opts.max_login_retry,
        keepalive_send_to: opts.keepalive_send_to,
        session: opts.session.clone(),
    };
    // println!("Session[2378]: {:?}", opts.session);


    ChatClient::new(params).run_forever();

    Ok(())
}

#[derive(Debug, Clone)]
enum PostType {
    Post(String, Option<String>),   // Message, SendTo
    Kick(String, String),           // Message, Username
    Upload(String, String, String), // FilePath, SendTo, Message
    DeleteLast,                     // DeleteLast
    DeleteAll,                      // DeleteAll
    NewNickname(String),            // NewUsername
    NewColor(String),               // NewColor
    Profile(String, String),        // NewColor, NewUsername
    Ignore(String),                 // Username
    Unignore(String),               // Username
    Clean(String, String),          // Clean message
}

// Get username of other user (or ours if it's the only one)
fn get_username(own_username: &str, root: &StyledText, members_tag: &str) -> Option<String> {
    match get_message(root, members_tag) {
        Some((from, Some(to), _)) => {
            if from == own_username {
                return Some(to);
            }
            return Some(from);
        }
        Some((from, None, _)) => {
            return Some(from);
        }
        _ => return None,
    }
}

// Extract "from"/"to"/"message content" from a "StyledText"
fn get_message(root: &StyledText, members_tag: &str ) -> Option<(String, Option<String>, String)> {
    if let StyledText::Styled(_, children) = root {
        let msg = children.get(0)?.text();
        match children.get(children.len() - 1)? {
            StyledText::Styled(_, children) => {
                let from = match children.get(children.len() - 1)? {
                    StyledText::Text(t) => t.to_owned(),
                    _ => return None,
                };
                return Some((from, None, msg));
            }
            StyledText::Text(t) => {
                if t == &members_tag {
                    let from = match children.get(children.len() - 2)? {
                        StyledText::Styled(_, children) => {
                            match children.get(children.len() - 1)? {
                                StyledText::Text(t) => t.to_owned(),
                                _ => return None,
                            }
                        }
                        _ => return None,
                    };
                    return Some((from, None, msg));
                } else if t == "[" {
                    let from = match children.get(children.len() - 2)? {
                        StyledText::Styled(_, children) => {
                            match children.get(children.len() - 1)? {
                                StyledText::Text(t) => t.to_owned(),
                                _ => return None,
                            }
                        }
                        _ => return None,
                    };
                    let to = match children.get(2)? {
                        StyledText::Styled(_, children) => {
                            match children.get(children.len() - 1)? {
                                StyledText::Text(t) => Some(t.to_owned()),
                                _ => return None,
                            }
                        }
                        _ => return None,
                    };
                    return Some((from, to, msg));
                }
            }
            _ => return None,
        }
    }
    return None;
}

#[derive(Debug, PartialEq, Clone)]
enum MessageType {
    UserMsg,
    SysMsg,
}

#[derive(Debug, PartialEq, Clone)]
struct Message {
    id: Option<usize>,
    typ: MessageType,
    date: String,
    upload_link: Option<String>,
    text: StyledText,
    deleted: bool, // Either or not a message was deleted on the chat
    hide: bool,    // Either ot not to hide a specific message
}

impl Message {
    fn new(
        id: Option<usize>,
        typ: MessageType,
        date: String,
        upload_link: Option<String>,
        text: StyledText,
    ) -> Self {
        Self {
            id,
            typ,
            date,
            upload_link,
            text,
            deleted: false,
            hide: false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum StyledText {
    Styled(tuiColor, Vec<StyledText>),
    Text(String),
    None,
}

impl StyledText {
    fn walk<F>(&self, mut clb: F)
    where
        F: FnMut(&StyledText),
    {
        let mut v: Vec<&StyledText> = vec![self];
        loop {
            if let Some(e) = v.pop() {
                clb(e);
                if let StyledText::Styled(_, children) = e {
                    v.extend(children);
                }
                continue;
            }
            break;
        }
    }

    fn text(&self) -> String {
        let mut s = String::new();
        self.walk(|n| {
            if let StyledText::Text(t) = n {
                s += t;
            }
        });
        s
    }

    // Return a vector of each text parts & what color it should be
    fn colored_text(&self) -> Vec<(tuiColor, String)> {
        let mut out: Vec<(tuiColor, String)> = vec![];
        let mut v: Vec<(tuiColor, &StyledText)> = vec![(tuiColor::White, self)];
        loop {
            if let Some((el_color, e)) = v.pop() {
                match e {
                    StyledText::Styled(tui_color, children) => {
                        for child in children {
                            v.push((*tui_color, child));
                        }
                    }
                    StyledText::Text(t) => {
                        out.push((el_color, t.to_owned()));
                    }
                    StyledText::None => {}
                }
                continue;
            }
            break;
        }
        out
    }
}

fn parse_color(color_str: &str) -> tuiColor {
    let mut color = tuiColor::White;
    if color_str == "red" {
        return tuiColor::Red;
    }
    if let Ok(rgb) = Rgb::from_hex_str(color_str) {
        color = tuiColor::Rgb(
            rgb.get_red() as u8,
            rgb.get_green() as u8,
            rgb.get_blue() as u8,
        );
    }
    color
}

fn process_node(e: select::node::Node, mut color: tuiColor) -> (StyledText, Option<String>) {
    match e.data() {
        select::node::Data::Element(_, _) => {
            let mut upload_link: Option<String> = None;
            match e.name() {
                Some("span") => {
                    if let Some(style) = e.attr("style") {
                        if let Some(captures) = COLOR_RGX.captures(style) {
                            let color_match = captures.get(1).unwrap().as_str();
                            color = parse_color(color_match);
                        }
                    }
                }
                Some("font") => {
                    if let Some(color_str) = e.attr("color") {
                        color = parse_color(color_str);
                    }
                }
                Some("a") => {
                    color = tuiColor::White;
                    if let (Some("attachement"), Some(href)) = (e.attr("class"), e.attr("href")) {
                        upload_link = Some(href.to_owned());
                    }
                }
                Some("style") => {
                    return (StyledText::None, None);
                }
                _ => {}
            }
            let mut children_texts: Vec<StyledText> = vec![];
            let children = e.children();
            for child in children {
                let (st, ul) = process_node(child, color);
                if ul.is_some() {
                    upload_link = ul;
                }
                children_texts.push(st);
            }
            children_texts.reverse();
            (StyledText::Styled(color, children_texts), upload_link)
        }
        select::node::Data::Text(t) => (StyledText::Text(t.to_string()), None),
        select::node::Data::Comment(_) => (StyledText::None, None),
    }
}

struct Users {
    admin: Vec<(tuiColor, String)>,
    staff: Vec<(tuiColor, String)>,
    members: Vec<(tuiColor, String)>,
    guests: Vec<(tuiColor, String)>,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            admin: Default::default(),
            staff: Default::default(),
            members: Default::default(),
            guests: Default::default(),
        }
    }
}

impl Users {
    fn all(&self) -> Vec<&(tuiColor, String)> {
        let mut out = Vec::new();
        out.extend(&self.admin);
        out.extend(&self.staff);
        out.extend(&self.members);
        out.extend(&self.guests);
        out
    }

    // fn is_guest(&self, name: &str) -> bool {
    //     self.guests.iter().find(|(_, username)| username == name).is_some()
    // }
}

fn extract_users(doc: &Document) -> Users {
    let mut users = Users::default();

    if let Some(chatters) = doc.find(Attr("id", "chatters")).next() {
        if let Some(tr) = chatters.find(Name("tr")).next() {
            let mut th_count = 0;
            for e in tr.children() {
                if let select::node::Data::Element(_, _) = e.data() {
                    if e.name() == Some("th") {
                        th_count += 1;
                        continue;
                    }
                    for user_span in e.find(Name("span")) {
                        if let Some(user_style) = user_span.attr("style") {
                            if let Some(captures) = COLOR_RGX.captures(user_style) {
                                if let Some(color_match) = captures.get(1) {
                                    let color = color_match.as_str().to_owned();
                                    let tui_color = parse_color(&color);
                                    let username = user_span.text();
                                    match th_count {
                                        1 => users.admin.push((tui_color, username)),
                                        2 => users.staff.push((tui_color, username)),
                                        3 => users.members.push((tui_color, username)),
                                        4 => users.guests.push((tui_color, username)),
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    users
}

fn remove_suffix<'a>(s: &'a str, suffix: &str) -> &'a str {
    s.strip_suffix(suffix).unwrap_or(s)
}

fn remove_prefix<'a>(s: &'a str, prefix: &str) -> &'a str {
    s.strip_prefix(prefix).unwrap_or(s)
}

fn extract_messages(doc: &Document) -> anyhow::Result<Vec<Message>> {
    Ok(doc.find(Attr("id", "messages"))
        .next()
        .ok_or_else(|| anyhow!("Gagal mendapatkan div pesan"))?
        .find(Attr("class", "msg"))
        .filter_map(|tag| {
            let id = tag.find(Name("input"))
                .next()
                .and_then(|checkbox| checkbox.attr("value"))
                .and_then(|value| value.parse().ok());

            let date_node = tag.find(Name("small")).next()?;
            let msg_span = tag.find(Name("span")).next()?;

            let date = remove_suffix(&date_node.text(), " - ").to_owned();
            let typ = match msg_span.attr("class") {
                Some("usermsg") => MessageType::UserMsg,
                Some("sysmsg") => MessageType::SysMsg,
                _ => return None,
            };

            let (text, upload_link) = process_node(msg_span, tuiColor::White);
            Some(Message::new(id, typ, date, upload_link, text))
        })
        .collect())
}

fn draw_terminal_frame(
    f: &mut Frame<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    messages: &Arc<Mutex<Vec<Message>>>,
    users: &Arc<Mutex<Users>>,
    username: &str,
) {
    if app.long_message.is_none() {
        let hchunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(25)].as_ref())
            .split(f.size());

        {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Min(1),
                    ]
                    .as_ref(),
                )
                .split(hchunks[0]);

            render_help_txt(f, app, chunks[0], username);
            render_textbox(f, app, chunks[1]);
            render_messages(f, app, chunks[2], messages);
            render_users(f, hchunks[1], users);
        }
    } else {
        let hchunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1)])
            .split(f.size());
        {
            render_long_message(f, app, hchunks[0]);
        }
    }
}

fn gen_lines(msg_txt: &StyledText, w: usize, line_prefix: &str) -> Vec<Vec<(tuiColor, String)>> {
    let txt = msg_txt.text();
    let wrapped = textwrap::fill(&txt, w);
    let splits = wrapped.split("\n").collect::<Vec<&str>>();
    let mut new_lines: Vec<Vec<(tuiColor, String)>> = Vec::new();
    let mut ctxt = msg_txt.colored_text();
    ctxt.reverse();
    let mut ptr = 0;
    let mut split_idx = 0;
    let mut line: Vec<(tuiColor, String)> = Vec::new();
    let mut first_in_line = true;
    loop {
        if let Some((color, mut txt)) = ctxt.pop() {
            txt = txt.replace("\n", "");
            if let Some(split) = splits.get(split_idx) {
                if let Some(chr) = txt.chars().next() {
                    if chr == ' ' && first_in_line {
                        let skipped: String = txt.chars().skip(1).collect();
                        txt = skipped;
                    }
                }

                let remain = split.len() - ptr;
                if txt.len() <= remain {
                    ptr += txt.len();
                    line.push((color, txt));
                    first_in_line = false;
                } else {
                    line.push((color, txt[0..remain].to_owned()));
                    new_lines.push(line.clone());
                    line.clear();
                    line.push((tuiColor::White, line_prefix.to_owned()));
                    ctxt.push((color, txt[(remain)..].to_owned()));
                    ptr = 0;
                    split_idx += 1;
                    first_in_line = true;
                }
            }
        } else {
            new_lines.push(line);
            break;
        }
    }
    new_lines
}

fn render_long_message(f: &mut Frame<CrosstermBackend<io::Stdout>>, app: &mut App, r: Rect) {
    if let Some(m) = &app.long_message {
        let new_lines = gen_lines(&m.text, (r.width - 2) as usize, "");

        let mut rows = vec![];
        for line in new_lines.into_iter() {
            let spans_vec: Vec<Span> = line
                .into_iter()
                .map(|(color, txt)| Span::styled(txt, Style::default().fg(color)))
                .collect();
            rows.push(Spans::from(spans_vec));
        }

        let messages_list_items = vec![ListItem::new(rows)];

        let messages_list = List::new(messages_list_items)
            .block(Block::default().borders(Borders::ALL).title(""))
            .highlight_style(
                Style::default()
                    .bg(tuiColor::Rgb(50, 50, 50))
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(messages_list, r);
    }
}

fn render_help_txt(
    f: &mut Frame<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    r: Rect,
    curr_user: &str,
) {
    let (mut msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("i", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default(),
        ),
        InputMode::Editing | InputMode::EditingErr => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
        InputMode::LongMessage => (vec![], Style::default()),
    };
    msg.extend(vec![Span::raw(format!(" | {}", curr_user))]);
    if app.is_muted {
        let fg = tuiColor::Red;
        let style = Style::default().fg(fg).add_modifier(Modifier::BOLD);
        msg.extend(vec![Span::raw(" | "), Span::styled("muted", style)]);
    } else {
        let fg = tuiColor::LightGreen;
        let style = Style::default().fg(fg).add_modifier(Modifier::BOLD);
        msg.extend(vec![Span::raw(" | "), Span::styled("not muted", style)]);
    }

    //Strange
    if app.display_guest_view {
        let fg = tuiColor::LightGreen;
        let style = Style::default().fg(fg).add_modifier(Modifier::BOLD);
        msg.extend(vec![Span::raw(" | "), Span::styled("G", style)]);
    } else {
        let fg = tuiColor::Gray;
        let style = Style::default().fg(fg);
        msg.extend(vec![Span::raw(" | "), Span::styled("G", style)]);
    }

    //Strange
    if app.display_member_view {
        let fg = tuiColor::LightGreen;
        let style = Style::default().fg(fg).add_modifier(Modifier::BOLD);
        msg.extend(vec![Span::raw(" | "), Span::styled("M", style)]);
    } else {
        let fg = tuiColor::Gray;
        let style = Style::default().fg(fg);
        msg.extend(vec![Span::raw(" | "), Span::styled("M", style)]);
    }

    if app.display_hidden_msgs {
        let fg = tuiColor::LightGreen;
        let style = Style::default().fg(fg).add_modifier(Modifier::BOLD);
        msg.extend(vec![Span::raw(" | "), Span::styled("H", style)]);
    } else {
        let fg = tuiColor::Gray;
        let style = Style::default().fg(fg);
        msg.extend(vec![Span::raw(" | "), Span::styled("H", style)]);
    }
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, r);
}

fn render_textbox(f: &mut Frame<CrosstermBackend<io::Stdout>>, app: &mut App, r: Rect) {
    let w = (r.width - 3) as usize;
    let str = app.input.clone();
    let mut input_str = str.as_str();
    let mut overflow = 0;
    if app.input_idx >= w {
        overflow = std::cmp::max(app.input.width() - w, 0);
        input_str = &str[overflow..];
    }
    let input = Paragraph::new(input_str)
        .style(match app.input_mode {
            InputMode::LongMessage => Style::default(),
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(tuiColor::Yellow),
            InputMode::EditingErr => Style::default().fg(tuiColor::Red),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, r);
    match app.input_mode {
        InputMode::LongMessage => {}
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing | InputMode::EditingErr => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                r.x + app.input_idx as u16 - overflow as u16 + 1,
                // Move one line down, from the border to the input line
                r.y + 1,
            )
        }
    }
}

fn render_messages(
    f: &mut Frame<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    r: Rect,
    messages: &Arc<Mutex<Vec<Message>>>,
) {
    // Messages
    app.items.items.clear();
    let messages = messages.lock().unwrap();
    let messages_list_items: Vec<ListItem> = messages
        .iter()
        .filter_map(|m| {
            if !app.display_hidden_msgs && m.hide {
                return None;
            }
            // Simulate a guest view (remove "PMs" and "Members chat" messages)
            if app.display_guest_view {
                // TODO: this is not efficient at all
                let text = m.text.text();
                if text.starts_with(&app.members_tag) || text.starts_with(&app.staffs_tag) {
                    return None;
                }
                if let Some((_, Some(_), _)) = get_message(&m.text, &app.members_tag) {
                    return None;
                }
            }

            // Strange
            // Display only messages from members and staff
            if app.display_member_view {
                // In members mode, include only messages from members and staff
                let text = m.text.text();
                if !text.starts_with(&app.members_tag) && !text.starts_with(&app.staffs_tag) {
                    return None;
                }
                if let Some((_, Some(_), _)) = get_message(&m.text, &app.members_tag) {
                    return None;
                }
            }

            if app.filter != "" {
                if !m
                    .text
                    .text()
                    .to_lowercase()
                    .contains(&app.filter.to_lowercase())
                {
                    return None;
                }
            }

            app.items.items.push(m.clone());

            let new_lines = gen_lines(&m.text, (r.width - 20) as usize, " ".repeat(17).as_str());

            let mut rows = vec![];
            let date_style = match (m.deleted, m.hide) {
                (false, true) => Style::default().fg(tuiColor::Gray),
                (false, _) => Style::default().fg(tuiColor::DarkGray),
                (true, _) => Style::default().fg(tuiColor::Red),
            };
            let mut spans_vec = vec![Span::styled(m.date.clone(), date_style)];
            let show_sys_sep = app.show_sys && m.typ == MessageType::SysMsg;
            let sep = if show_sys_sep { " * " } else { " -=- " };
            spans_vec.push(Span::raw(sep));
            for (idx, line) in new_lines.into_iter().enumerate() {
                // Spams can take your whole screen, so we limit to 5 lines.
                if idx >= 5 {
                    spans_vec.push(Span::styled(
                        "                 […]",
                        Style::default().fg(tuiColor::White),
                    ));
                    rows.push(Spans::from(spans_vec));
                    break;
                }
                for (color, txt) in line {
                    spans_vec.push(Span::styled(txt, Style::default().fg(color)));
                }
                rows.push(Spans::from(spans_vec.clone()));
                spans_vec.clear();
            }

            let style = match (m.deleted, m.hide) {
                (true, _) => Style::default().bg(tuiColor::Rgb(30, 0, 0)),
                (_, true) => Style::default().bg(tuiColor::Rgb(20, 20, 20)),
                _ => Style::default(),
            };
            Some(ListItem::new(rows).style(style))
        })
        .collect();

    let messages_list = List::new(messages_list_items)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .highlight_style(
            Style::default()
                .bg(tuiColor::Rgb(50, 50, 50))
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(messages_list, r, &mut app.items.state)
}

fn render_users(f: &mut Frame<CrosstermBackend<io::Stdout>>, r: Rect, users: &Arc<Mutex<Users>>) {
    // Users lists
    let users = users.lock().unwrap();
    let mut users_list: Vec<ListItem> = vec![];
    let mut users_types: Vec<(&Vec<(tuiColor, String)>, &str)> = Vec::new();
    users_types.push((&users.admin, "-- Admin --"));
    users_types.push((&users.staff, "-- Staff --"));
    users_types.push((&users.members, "-- Members --"));
    users_types.push((&users.guests, "-- Guests --"));

    for (user_group, label) in users_types.iter_mut() {
        users_list.push(ListItem::new(Span::raw(*label)));
        for (tui_color, username) in user_group.iter() {
            let span = Span::styled(username, Style::default().fg(*tui_color));
            users_list.push(ListItem::new(span));
        }
    }

    let users_widget = List::new(users_list).block(Block::default().borders(Borders::ALL).title("Users"));
    f.render_widget(users_widget, r);
}

fn random_string(n: usize) -> String {
    let s: Vec<u8> = thread_rng().sample_iter(&Alphanumeric).take(n).collect();
    std::str::from_utf8(&s).unwrap().to_owned()
}

#[derive(PartialEq)]
enum InputMode {
    LongMessage,
    Normal,
    Editing,
    EditingErr,
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    input_idx: usize,
    /// Current input mode
    input_mode: InputMode,
    is_muted: bool,
    show_sys: bool,
    display_guest_view: bool,
    display_member_view: bool,
    display_hidden_msgs: bool,
    items: StatefulList<Message>,
    filter: String,
    members_tag: String,
    staffs_tag: String,
    long_message: Option<Message>,
    commands: Commands,
}

impl Default for App {
    fn default() -> App {
        // Read commands from the file and set them as default values
        let commands = if let Ok(config_path) = confy::get_configuration_file_path("bhcli", None) {
            if let Some(config_path_str) = config_path.to_str() {
                match read_commands_file(config_path_str) {
                    Ok(commands) => commands,
                    Err(err) => {
                        log::error!(
                            "Failed to read commands from config file - {} :
{}",
                            config_path_str,
                            err
                        );
                        Commands {
                            commands: HashMap::new(),
                        }
                    }
                }
            } else {
                log::error!("Failed to convert configuration file path to string.");
                Commands {
                    commands: HashMap::new(),
                }
            }
        } else {
            log::error!("Failed to get configuration file path.");
            Commands {
                commands: HashMap::new(),
            }
        };

        App {
            input: String::new(),
            input_idx: 0,
            input_mode: InputMode::Normal,
            is_muted: false,
            show_sys: false,
            display_guest_view: false,
            display_member_view: false,
            display_hidden_msgs: false,
            items: StatefulList::new(),
            filter: "".to_owned(),
            members_tag: "".to_owned(),
            staffs_tag: "".to_owned(),
            long_message: None,
            commands,
        }
    }
}

impl App {
    fn update_filter(&mut self) {
        if let Some(captures) = FIND_RGX.captures(&self.input) {
            // Find
            self.filter = captures.get(1).map_or("", |m| m.as_str()).to_owned();
        }
    }

    fn clear_filter(&mut self) {
        if FIND_RGX.is_match(&self.input) {
            self.filter = "".to_owned();
            self.input = "".to_owned();
            self.input_idx = 0;
        }
    }
}

pub enum Event<I> {
    Input(I),
    Tick,
    Terminate,
    NeedLogin,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
struct Events {
    messages_updated_rx: crossbeam_channel::Receiver<()>,
    exit_rx: crossbeam_channel::Receiver<ExitSignal>,
    rx: crossbeam_channel::Receiver<Event<CEvent>>,
}

#[derive(Debug, Clone)]
struct Config {
    pub exit_rx: crossbeam_channel::Receiver<ExitSignal>,
    pub messages_updated_rx: crossbeam_channel::Receiver<()>,
    pub tick_rate: Duration,
}

impl Events {
    fn with_config(config: Config) -> (Events, thread::JoinHandle<()>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let tick_rate = config.tick_rate;
        let exit_rx = config.exit_rx;
        let messages_updated_rx = config.messages_updated_rx;
        let exit_rx1 = exit_rx.clone();
        let thread_handle = thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                // poll for tick rate duration, if no events, sent tick event.
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));
                if event::poll(timeout).unwrap() {
                    let evt = event::read().unwrap();
                    match evt {
                        CEvent::FocusGained => {}
                        CEvent::FocusLost => {}
                        CEvent::Paste(_) => {}
                        CEvent::Resize(_, _) => tx.send(Event::Input(evt)).unwrap(),
                        CEvent::Key(_) => tx.send(Event::Input(evt)).unwrap(),
                        CEvent::Mouse(mouse_event) => {
                            match mouse_event.kind {
                                MouseEventKind::ScrollDown
                                | MouseEventKind::ScrollUp
                                | MouseEventKind::Down(_) => {
                                    tx.send(Event::Input(evt)).unwrap();
                                }
                                _ => {}
                            };
                        }
                    };
                }
                if last_tick.elapsed() >= tick_rate {
                    select! {
                        recv(&exit_rx1) -> _ => break,
                        default => {},
                    }
                    last_tick = Instant::now();
                }
            }
        });
        (
            Events {
                rx,
                exit_rx,
                messages_updated_rx,
            },
            thread_handle,
        )
    }

    fn next(&self) -> Result<Event<CEvent>, crossbeam_channel::RecvError> {
        select! {
            recv(&self.rx) -> evt => evt,
            recv(&self.messages_updated_rx) -> _ => Ok(Event::Tick),
            recv(&self.exit_rx) -> v => match v {
                Ok(ExitSignal::Terminate) => Ok(Event::Terminate),
                Ok(ExitSignal::NeedLogin) => Ok(Event::NeedLogin),
                Err(_) => Ok(Event::Terminate),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_lines_test() {
        let txt = StyledText::Styled(
            tuiColor::White,
            vec![
                StyledText::Styled(
                    tuiColor::Rgb(255, 255, 255),
                    vec![
                        StyledText::Text(" prmdbba pwuv💓".to_owned()),
                        StyledText::Styled(
                            tuiColor::Rgb(255, 255, 255),
                            vec![StyledText::Styled(
                                tuiColor::Rgb(0, 255, 0),
                                vec![StyledText::Text("PMW".to_owned())],
                            )],
                        ),
                        StyledText::Styled(
                            tuiColor::Rgb(255, 255, 255),
                            vec![StyledText::Styled(
                                tuiColor::Rgb(255, 255, 255),
                                vec![StyledText::Text("A".to_owned())],
                            )],
                        ),
                        StyledText::Styled(
                            tuiColor::Rgb(255, 255, 255),
                            vec![StyledText::Styled(
                                tuiColor::Rgb(0, 255, 0),
                                vec![StyledText::Text("XOS".to_owned())],
                            )],
                        ),
                        StyledText::Text(
                            "pqb a mavx pkj fhsoeycg oruzb asd lk ruyaq re lheot mbnrw ".to_owned(),
                        ),
                    ],
                ),
                StyledText::Text(" - ".to_owned()),
                StyledText::Styled(
                    tuiColor::Rgb(255, 255, 255),
                    vec![StyledText::Text("rytxvgs".to_owned())],
                ),
            ],
        );
        let lines = gen_lines(&txt, 71, "");
        assert_eq!(lines.len(), 2);
    }
}
