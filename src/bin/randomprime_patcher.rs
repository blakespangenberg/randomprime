use clap::{
    Arg,
    ArgGroup,
    App,
    Format, // XXX This is an undocumented enum
    crate_version,
};

use randomprime::{
    extract_flaahgra_music_files, patches, reader_writer,
    starting_items::StartingItems, structs,
};

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    fs,
    panic,
    process::Command,
};

use serde::{Deserialize};


struct ProgressNotifier
{
    total_size: usize,
    bytes_so_far: usize,
    quiet: bool,
}

impl ProgressNotifier
{
    fn new(quiet: bool) -> ProgressNotifier
    {
        ProgressNotifier {
            total_size: 0,
            bytes_so_far: 0,
            quiet,
        }
    }
}

impl structs::ProgressNotifier for ProgressNotifier
{
    fn notify_total_bytes(&mut self, total_size: usize)
    {
        self.total_size = total_size
    }

    fn notify_writing_file(&mut self, file_name: &reader_writer::CStr, file_bytes: usize)
    {
        if self.quiet {
            return;
        }
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        println!("{:02.0}% -- Writing file {:?}", percent, file_name);
        self.bytes_so_far += file_bytes;
    }

    fn notify_writing_header(&mut self)
    {
        if self.quiet {
            return;
        }
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        println!("{:02.0}% -- Writing ISO header", percent);
    }

    fn notify_flushing_to_disk(&mut self)
    {
        if self.quiet {
            return;
        }
        println!("Flushing written data to the disk...");
    }

    fn notify_stacking_warning(&mut self)
    {
        if self.quiet {
            return;
        }
        println!("Item randomized game. Skipping item randomizer configuration.");
    }
}

fn default_as_false() -> bool {
    false
}

fn default_as_empty_str_vec() -> Vec<String> {
    Vec::new()
}

fn default_as_empty_bool_vec() -> Vec<bool> {
    Vec::new()
}

fn default_as_empty_liquid_volume_vec() -> Vec<patches::LiquidVolume> {
    Vec::new()
}

fn default_as_empty_aether_transform_vec() -> Vec<patches::AetherTransform> {
    Vec::new()
}

fn default_as_empty_add_items_vec() -> Vec<patches::AdditionalItem> {
    Vec::new()
}

fn default_empty_string() -> String {
    "".to_string()
}

fn default_u64_123456789() -> u64 {
    123456789
}

#[derive(Deserialize)]
struct PatchConfig {
    skip_crater: bool,
    fix_flaaghra_music: bool,
    trilogy_iso: Option<String>,
    varia_heat_protection: bool,
    stagger_suit_damage: bool,
    skip_hudmemos: bool,
    powerbomb_lockpick: bool,
    enable_one_way_doors: bool,
    patch_map: bool,
    obfuscate_items:bool,
    artifact_hints:String,
    auto_enabled_elevators:bool,
    
    #[serde(default = "default_as_false")]
    patch_vertical_to_blue:bool,
    
    #[serde(default = "default_as_false")]
    patch_power_conduits: bool,

    #[serde(default = "default_as_false")]
    tiny_elvetator_samus: bool,

    #[serde(default = "default_as_false")]
    remove_missile_locks: bool,

    #[serde(default = "default_as_false")]
    remove_frigidite_lock: bool,

    #[serde(default = "default_as_false")]
    remove_mine_security_station_locks: bool,

    #[serde(default = "default_as_false")]
    lower_mines_backwards: bool,

    #[serde(default = "default_as_false")]
    biohazard_containment_alt_spawn: bool,

    #[serde(default = "default_as_false")]
    remove_hall_of_the_elders_forcefield: bool,

    #[serde(default = "default_as_false")]
    quickplay: bool,
}

#[derive(Deserialize)]
struct Config {
    input_iso: String,
    output_iso: String,
    layout_string: String,

    #[serde(default = "default_as_empty_str_vec")]
    elevator_layout_override: Vec<String>,

    #[serde(default = "default_as_empty_bool_vec")]
    missile_lock_override: Vec<bool>,

    #[serde(default = "default_as_empty_str_vec")]
    superheated_rooms: Vec<String>,

    #[serde(default = "default_as_empty_str_vec")]
    deheated_rooms: Vec<String>,

    #[serde(default = "default_as_empty_str_vec")]
    drain_liquid_rooms: Vec<String>,

    #[serde(default = "default_as_empty_str_vec")]
    underwater_rooms: Vec<String>,

    #[serde(default = "default_as_empty_liquid_volume_vec")]
    liquid_volumes: Vec<patches::LiquidVolume>,

    #[serde(default = "default_as_empty_aether_transform_vec")]
    aether_transforms: Vec<patches::AetherTransform>,

    #[serde(default = "default_as_empty_add_items_vec")]
    additional_items: Vec<patches::AdditionalItem>,
    
    #[serde(default = "default_empty_string")]
    new_save_spawn_room: String,

    #[serde(default = "default_empty_string")]
    frigate_done_spawn_room: String,

    seed: u64,
    patch_settings: PatchConfig,

    new_save_starting_items: u64,
    frigate_done_starting_items: u64,
    
    excluded_doors: [HashMap<String,Vec<String>>;7],
}

#[derive(Deserialize)]
struct ConfigBanner
{
    game_name: Option<String>,
    developer: Option<String>,

    game_name_full: Option<String>,
    developer_full: Option<String>,
    description: Option<String>,
}

fn get_config() -> Result<patches::ParsedConfig, String>
{
    /*let matches = App::new("randomprime ISO patcher")
        .version(crate_version!())
        .arg(Arg::with_name("input iso path")
            .long("input-iso")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("output iso path")
            .long("output-iso")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("pickup layout")
            .long("layout")
            .required(true)
            .takes_value(true)
            .allow_hyphen_values(true))*/


    let matches = App::new("randomprime ISO patcher")
        .version(crate_version!())
        .arg(Arg::with_name("input iso path")
            .long("input-iso")
            .takes_value(true))
        .arg(Arg::with_name("output iso path")
            .long("output-iso")
            .takes_value(true))
        .arg(Arg::with_name("profile json path")
            .long("profile")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("skip frigate")
            .long("skip-frigate")
            .help("New save files will skip the \"Space Pirate Frigate\" tutorial level"))
        .arg(Arg::with_name("skip hudmenus")
            .long("non-modal-item-messages")
            .help("Display a non-modal message when an item is is acquired"))
        .arg(Arg::with_name("etank capacity")
            .long("etank-capacity")
            .help("Set the etank capacity and base health")
            .takes_value(true))
        .arg(Arg::with_name("nonvaria heat damage")
            .long("nonvaria-heat-damage")
            .help("If the Varia Suit has not been collect, heat damage applies"))
        .arg(Arg::with_name("heat damage per sec")
            .long("heat-damage-per-sec")
            .help("Set the heat damage per seconds spent in a superheated room")
            .takes_value(true))
        .arg(Arg::with_name("staggered suit damage")
            .long("staggered-suit-damage")
            .help(concat!("The suit damage reduction is determinted by the number of suits ",
                            "collected rather than the most powerful one collected.")))
        .arg(Arg::with_name("max obtainable missiles")
            .long("max-obtainable-missiles")
            .help("Set the max amount of Missiles you can carry")
            .takes_value(true))
        .arg(Arg::with_name("max obtainable power bombs")
            .long("max-obtainable-power-bombs")
            .help("Set the max amount of Power Bombs you can carry")
            .takes_value(true))
        .arg(Arg::with_name("auto enabled elevators")
            .long("auto-enabled-elevators")
            .help("Every elevator will be automatically enabled without scaning its terminal"))
        .arg(Arg::with_name("skip impact crater")
            .long("skip-impact-crater")
            .help("Elevators to the Impact Crater immediately go to the game end sequence"))
        .arg(Arg::with_name("enable vault ledge door")
            .long("enable-vault-ledge-door")
            .help("Enable Chozo Ruins Vault door from Main Plaza"))

        .arg(Arg::with_name("all artifact hints")
            .long("all-artifact-hints")
            .help("All artifact location hints are available immediately"))
        .arg(Arg::with_name("no artifact hints")
            .long("no-artifact-hints")
            .help("Artifact location hints are disabled"))
        .group(ArgGroup::with_name("artifact hint behavior")
               .args(&["all artifact hints", "no artifact hints"]))

        .arg(Arg::with_name("trilogy disc path")
            .long("flaahgra-music-disc-path")
            .help(concat!("Location of a ISO of Metroid Prime Trilogy. If provided the ",
                            "Flaahgra fight music will be used to replace the original"))
            .takes_value(true))
        .arg(Arg::with_name("suit hue rotate angle")
            .long("suit-hue-rotate-angle")
            .takes_value(true)
            .validator(|s| s.parse::<i32>().map(|_| ())
                                        .map_err(|_| "Expected an integer".to_string())))
        .arg(Arg::with_name("keep attract mode")
            .long("keep-attract-mode")
            .help("Keeps the attract mode FMVs, which are removed by default"))
        .arg(Arg::with_name("obfuscate items")
            .long("obfuscate-items")
            .help("Replace all item models with an obfuscated one"))
        .arg(Arg::with_name("quiet")
            .long("quiet")
            .help("Don't print the progress messages"))
        .arg(Arg::with_name("main menu message")
            .long("main-menu-message")
            .hidden(true)
            .takes_value(true))
        .arg(Arg::with_name("random starting items")
            .long("random-starting-items")
            .hidden(true)
            .takes_value(true)
            .validator(|s| s.parse::<u64>().map(|_| ())
                                        .map_err(|_| "Expected an integer".to_string())))
        .arg(Arg::with_name("change starting items")
            .long("starting-items")
            .hidden(true)
            .takes_value(true)
            .validator(|s| s.parse::<u64>().map(|_| ())
                                        .map_err(|_| "Expected an integer".to_string())))
        .arg(Arg::with_name("quickplay")
            .long("quickplay")
            .hidden(true))
        .arg(Arg::with_name("text file comment")
                .long("text-file-comment")
                .hidden(true)
                .takes_value(true))
        .get_matches();

    let json_path = matches.value_of("profile json path").unwrap();
    let input_json:&str = &fs::read_to_string(json_path)
                .map_err(|e| format!("Could not read JSON file: {}",e)).unwrap();

    let config:Config = serde_json::from_str(input_json)
                .map_err(|e| format!("Could not parse JSON file: {}",e)).unwrap();
    let input_iso_path = config.input_iso;
    let input_iso_file = File::open(input_iso_path)
                .map_err(|e| format!("Failed to open input iso: {}", e))?;
    let input_iso_mmap = unsafe { memmap::Mmap::map(&input_iso_file) }
                .map_err(|e| format!("Failed to open input iso: {}", e))?;

    let output_iso_path = config.output_iso;
    let out_iso = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&output_iso_path)
        .map_err(|e| format!("Failed to open output file: {}", e))?;

    let iso_format = if output_iso_path.ends_with(".gcz") {
        patches::IsoFormat::Gcz
    } else if output_iso_path.ends_with(".ciso") {
        patches::IsoFormat::Ciso
    } else {
        patches::IsoFormat::Iso
    };

    let layout = matches.value_of("pickup layout").unwrap().parse()?;

    let seed = config.seed;

    let artifact_hints = String::from(&config.patch_settings.artifact_hints);
    let artifact_hint_behavior = if artifact_hints == "default" {
        patches::ArtifactHintBehavior::Default
    } else if artifact_hints == "none" {
        patches::ArtifactHintBehavior::None
    } else { // e.g. "all"
        patches::ArtifactHintBehavior::All
        
    };

    let flaahgra_music_files = if config.patch_settings.fix_flaaghra_music {
        if let Some(path) = config.patch_settings.trilogy_iso {
            Some(extract_flaahgra_music_files(&path)?)
        } else {
            None
        }
    } else {
        None
    };

    let new_save_starting_items = matches.value_of("new save starting items")
        .map(|s| StartingItems::from_u64(s.parse().unwrap()))
        .unwrap_or(StartingItems::from_u64(0));

    let frigate_done_starting_items = matches.value_of("frigate done starting items")
        .map(|s| StartingItems::from_u64(s.parse().unwrap()))
        .unwrap_or(StartingItems::from_u64(0));

    Ok(patches::ParsedConfig {
        input_iso: input_iso_mmap,
        output_iso: out_iso,

        additional_items: vec![].into(),
        aether_transforms: vec![].into(),
        deheated_rooms: vec![].into(),
        superheated_rooms: vec![].into(),
        drain_liquid_rooms: vec![].into(),
        liquid_volumes: vec![].into(),
        underwater_rooms: vec![].into(),
        missile_lock_override: vec![].into(),
        
        excluded_doors: config.excluded_doors,
        new_save_spawn_room: config.new_save_spawn_room,
        frigate_done_spawn_room: config.frigate_done_spawn_room,
        new_save_starting_items,
        frigate_done_starting_items,

        biohazard_containment_alt_spawn: false,
        powerbomb_lockpick: false,
        remove_frigidite_lock: false,
        patch_power_conduits: false,
        lower_mines_backwards: false,
        remove_hall_of_the_elders_forcefield: false,
        remove_mine_security_station_locks: false,
        remove_missile_locks: false,

        layout,

        iso_format,
        skip_hudmenus: matches.is_present("skip hudmenus"),
        etank_capacity: matches.value_of("etank capacity")
                                    .unwrap_or_default()
                                    .parse::<u32>()
                                    .unwrap_or(100),
        nonvaria_heat_damage: matches.is_present("nonvaria heat damage"),
        heat_damage_per_sec: matches.value_of("heat damage per sec")
                                    .unwrap_or_default()
                                    .parse::<f32>()
                                    .unwrap_or(10.0),
        staggered_suit_damage: matches.is_present("staggered suit damage"),
        max_obtainable_missiles: matches.value_of("max obtainable missiles")
                                    .unwrap_or_default()
                                    .parse::<u32>()
                                    .unwrap_or(250),
        max_obtainable_power_bombs: matches.value_of("max obtainable power bombs")
                                    .unwrap_or_default()
                                    .parse::<u32>()
                                    .unwrap_or(8),
        keep_fmvs: matches.is_present("keep attract mode"),
        obfuscate_items: matches.is_present("obfuscate items"),
        auto_enabled_elevators: matches.is_present("auto enabled elevators"),
        quiet: matches.is_present("quiet"),
        enable_vault_ledge_door: matches.is_present("enable vault ledge door"),

        artifact_hint_behavior,
        tiny_elvetator_samus: config.patch_settings.tiny_elvetator_samus,

        flaahgra_music_files,
        suit_hue_rotate_angle: matches.value_of("suit hue rotate angle")
                .map(|s| s.parse::<i32>().unwrap()),
                               
        comment: matches.value_of("text file comment").unwrap_or("").to_string(),
        main_menu_message: matches.value_of("main menu message").unwrap_or("").to_string(),

        quickplay: config.patch_settings.quickplay,
        
        bnr_game_name: None,
        bnr_developer: None,

        bnr_game_name_full: None,
        bnr_developer_full: None,
        bnr_description: None,
    })
}

#[cfg(windows)]
fn was_launched_by_windows_explorer() -> bool
{
    // https://stackoverflow.com/a/513574
    use winapi::um::processenv:: *;
    use winapi::um::winbase:: *;
    use winapi::um::wincon:: *;
    static mut CACHED: Option<bool> = None;
    unsafe {
        if let Some(t) = CACHED {
            return t;
        }
        let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        let x = GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &mut csbi);
        CACHED = Some(x == 1 && csbi.dwCursorPosition.X == 0 && csbi.dwCursorPosition.Y == 0);
        CACHED.unwrap()
    }
}

#[cfg(not(windows))]
fn was_launched_by_windows_explorer() -> bool
{
    false
}

fn maybe_pause_at_exit()
{
    if was_launched_by_windows_explorer() {
        // XXX Windows only
        let _ = Command::new("cmd.exe").arg("/c").arg("pause").status();
    }
}

fn main_inner() -> Result<(), String>
{
    let config = get_config()?;
    let pn = ProgressNotifier::new(config.quiet);
    patches::patch_iso(config, pn)?;
    println!("Done");
    Ok(())
}

fn main()
{
    // XXX We have to check this before we print anything; it relies on the cursor position and
    //     caches its result.
    was_launched_by_windows_explorer();

    // On non-debug builds, suppress the default panic message and print a more helpful and
    // user-friendly one
    if !cfg!(debug_assertions) {
        panic::set_hook(Box::new(|_| {
            let _ = eprintln!("{} \
An error occurred while parsing the input ISO. \
This most likely means your ISO is corrupt. \
Please verify that your ISO matches one of the following hashes:
MD5:  eeacd0ced8e2bae491eca14f141a4b7c
SHA1: ac20c744db18fdf0339f37945e880708fd317231
", Format::Error("error:"));

            maybe_pause_at_exit();
        }));
    }

    match main_inner() {
        Err(s) => eprintln!("{} {}", Format::Error("error:"), s),
        Ok(()) => (),
    };

    maybe_pause_at_exit();
}
