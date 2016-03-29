#[warn(unused_must_use)]
extern crate hyper;
extern crate rustc_serialize;
extern crate time;

use std::io;
use std::io::Read;
use std::thread;
use std::process::Command;
use rustc_serialize::json;
use std::cmp::Ordering;
use hyper::Client;
use hyper::header::Connection;
use std::fs::OpenOptions;



#[derive(Default)]
struct Setting {
    min_d: u32,
    max_d: u32,
}

impl Setting {
    fn set_min_d(&mut self, min_d: &str) {
        let num = min_d.trim().parse::<u32>();
        match num {
            Ok(num) => self.min_d = num * 60 * 1000,
            Err(_) => self.min_d = 0 as u32,
        }

    }
    fn set_max_d(&mut self, max_d: &str) {
        let num = max_d.trim().parse::<u32>();
        match num {
            Ok(num) => self.max_d = num * 60 * 1000,
            Err(_) => self.max_d = 0 as u32,
        }

    }
}

// represents a soundcloud user account
#[derive(RustcDecodable)]
#[allow(dead_code)]
struct SoundCloudUser {
    id: u32,
    username: Option<String>,
    city: Option<String>,
    website: Option<String>,
    full_name: Option<String>,
}

// represents a search result
#[derive(RustcDecodable)]
#[allow(dead_code)]
struct SearchResult {
    title: String,
    created_at: String,
    duration: u32,
    stream_url: Option<String>,
    description: Option<String>,
    permalink_url: Option<String>,
    download_url: Option<String>,
    user: SoundCloudUser,
    created_at_formated: Option<String>,
    downloadable: bool,
}

impl std::cmp::Ord for SearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.duration > other.duration {
            Ordering::Greater
        } else if self.duration < other.duration {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}
impl std::cmp::PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.duration > other.duration {
            Some(Ordering::Greater)
        } else if self.duration < other.duration {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}
impl std::cmp::Eq for SearchResult {}
impl std::cmp::PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        if self.duration == other.duration {
            true
        } else {
            false
        }
    }
}

type SearchResults = Vec<SearchResult>;

struct Player {
    input: String,
    srs: SearchResults,
    setting: Setting,
    client_id: String,
    vlc_process_id: u32,
}

impl Player {
    fn search(&mut self, search_string: &str) {
        println!("Searching for '{}'.", search_string);
        let client = Client::new();
        let query = format!("https://api.soundcloud.com/tracks.json?\
            client_id={}&\
            q={}&\
            duration[from]={}&\
            duration[to]={}&\
            filter=streamable,public",
                            self.client_id,
                            search_string,
                            &self.setting.min_d,
                            &self.setting.max_d);
        let mut res = client.get(&query)
                            .header(Connection::close())
                            .send()
                            .unwrap();
        if res.status == hyper::status::StatusCode::Ok {
            let mut body = String::new();
            res.read_to_string(&mut body).unwrap();
            self.srs = json::decode(&body).unwrap();
            self.srs.sort();
        } else {
            println!("Server is not ok. The status is: '{}'", res.status);
        }
    }

    fn set(&mut self, setting_string: &str) {
        let mut iter = setting_string.split_whitespace();
        match iter.next() {
            Some(attribute) => {
                match attribute {
                    "range" => {
                        &self.setting.set_min_d(iter.next().unwrap_or("3"));
                        &self.setting.set_max_d(iter.next().unwrap_or("500"));
                    }
                    _ => (),
                }
            }
            None => {}
        }
    }

    fn kill_and_play(&mut self, num: usize) {
        self.show_dl_links(num);
        if self.srs.get(num - 1).unwrap().downloadable {
            self.play_af(num);
        } else {
            self.play_vlc(num);
        }
    }

    fn show_dl_links(&mut self, num: usize) {
        println!("Playing: {}", &self.srs.get(num - 1).unwrap().title);
        println!("Link: {}",
                 &self.srs
                      .get(num - 1)
                      .unwrap()
                      .permalink_url
                      .as_ref()
                      .unwrap_or(&String::from("none")));
        println!("Stream: {}",
                 &self.srs
                      .get(num - 1)
                      .unwrap()
                      .stream_url
                      .as_ref()
                      .unwrap_or(&String::from("none")));
        println!("Download: {}",
                 &self.srs
                      .get(num - 1)
                      .unwrap()
                      .download_url
                      .as_ref()
                      .unwrap_or(&String::from("none")));
    }

    fn play_af(&mut self, num: usize) {
        let child: std::process::Child;

        let durl = format!("{}?client_id={}",
                           &self.srs
                                .get(num - 1)
                                .unwrap()
                                .download_url
                                .as_ref()
                                .unwrap_or(&String::from("")),
                           &self.client_id);
        let client = Client::new();
        let mut res = client.get(&durl)
                            .header(Connection::close())
                            .send()
                            .unwrap();
        if res.status == hyper::status::StatusCode::Ok {
            let option = OpenOptions::new()
                             .read(true)
                             .write(true)
                             .create(true)
                             .open("/tmp/scpfile");
            match option {
                Ok(mut f) => {
                    thread::spawn(move || {
                        io::copy(&mut res, &mut f).unwrap();
                        println!("finished download");
                    });
                    thread::sleep(std::time::Duration::new(1, 234_567_890));
                    println!("continue");
                    child = Command::new("afplay")
                                .arg("/tmp/scpfile")
                                .spawn()
                                .unwrap_or_else(|e| panic!("failed to execute child: {}", e));
                    self.kill();
                    self.vlc_process_id = child.id();
                }
                Err(e) => println!("error openin file: {:?}", e),
            };
        } else {
            println!("{:?}", res.status);
        }
    }

    fn play_vlc(&mut self, num: usize) {
        let surl = format!("{}?client_id={}",
                           &self.srs
                                .get(num - 1)
                                .unwrap()
                                .stream_url
                                .as_ref()
                                .unwrap_or(&String::from("")),
                           &self.client_id);

        let child = Command::new("/Applications/VLC.app/Contents/MacOS/VLC")
                        .arg(surl)
                        .spawn()
                        .unwrap_or_else(|e| panic!("failed to execute child: {}", e));
        println!("Streaming 128kbps ... bummer!");
        self.kill();
        self.vlc_process_id = child.id();
    }

    fn kill(&mut self) {
        if self.vlc_process_id != 0 {
            let com = Command::new("kill")
                          .arg("-9")
                          .arg(format!("{}", self.vlc_process_id))
                          .spawn();
            match com {
                Ok(_) => (),
                Err(e) => println!("Error killing {:?}", e),
            }
        }
    }

    fn show_result_list(&mut self) {
        if self.srs.len() == 0 {
            println!("no results found");
        }
        let mut counter = 1;
        for item in self.srs.iter() {
            let &SearchResult {ref title,  ref created_at, ref downloadable, ref description, ref duration, ref user, ..} = item;

            // format rank numbers
            let rank: String;
            if counter <= 9 {
                rank = format!(" {}", counter);
            } else {
                rank = format!("{}", counter);
            }

            // indicate description using info symbol '[i]'
            let desc_avail: &str;
            match description.clone() {
                Some(_) => {
                    desc_avail = "\x1b[33m[i]\x1b[0m";
                }
                None => {
                    desc_avail = "   ";
                }
            }

            let download_avail: &str;
            if *downloadable {
                download_avail = "\x1b[33m[d]\x1b[0m";
            } else {
                download_avail = "   ";
            }

            println!("{}. {}{} {}->{} {} {}",
                     rank,
                     desc_avail,
                     download_avail,
                     title,
                     user.username.clone().unwrap(),
                     time::strptime(created_at, "%Y/%m/%d %H:%M:%S %z")
                         .unwrap()
                         .strftime("%d.%m.%y")
                         .unwrap(),
                     format!("{}min", duration / 60_000),
                     );
            counter += 1;
        }
    }

    fn show_track_info(&mut self, num: usize) {
        match self.srs.get(num - 1).unwrap().description {
            Some(ref d) => println!("{:?}", d),
            None => (), 
        }
    }

    fn dispatch(&mut self, input: &str) -> bool {
        self.input = String::from(input.trim());

        // maybe a track to play?
        let num = input.trim().parse::<usize>();
        match num {
            Ok(val) => {
                self.kill_and_play(val);
                return true;
            }
            Err(_) => (), // an error here is kind of a success to ;)
        }

        // maybe some setting to set?
        if input.len() >= 4 && &input[..4] == "set " {
            self.set(&input.trim()[3..]);
            return true;
        }

        // maybe a result list to show?
        if input.len() >= 2 && &input[..2] == "ll" {
            self.show_result_list();
            return true;
        }

        // maybe show the info of a track?
        if input.len() >= 3 && &input[..2] == "i " {
            self.show_track_info(input[2..]
                                     .trim()
                                     .parse::<usize>()
                                     .unwrap());
            return true;
        }

        // maybe quitting the program?
        if input == "x\n" {
            self.kill();
            println!("Bye");
            return false;
        }

        // if nothing else it must be a search term
        self.search(input.trim());
        self.show_result_list();
        true
    }
}

fn main() {
    let mut input = String::new();
    let srs: Vec<SearchResult> = Vec::new();
    let setting = Setting {
        min_d: 3 * 60 * 1000,
        max_d: 5 * 60 * 1000,
    };

    let settings_file = format!("{}/.soccli2",
                                std::env::home_dir().unwrap().to_str().unwrap());

    let opt_file = OpenOptions::new()
                       .read(true)
                       .open(settings_file);

    let mut settings_file_content: String = String::from("");

    match opt_file {
        Ok(mut f) => {
            f.read_to_string(&mut settings_file_content);
        }
        Err(e) => {
            println!("Please put your Soundcloud key into a file called '.soccli2' in your home \
                      directory. {}",
                     e);
            return;
        }
    }


    let mut p = Player {
        srs: srs,
        input: input.clone(),
        setting: setting,
        client_id: settings_file_content,
        vlc_process_id: 0,
    };

    loop {
        input.clear();
        let result = io::stdin().read_line(&mut input);
        match result {
            Ok(_) => {
                if !p.dispatch(&input) {
                    break;
                }
            }
            Err(e) => panic!("Could not read from stdin: {:?}", e),
        }
    }
}
