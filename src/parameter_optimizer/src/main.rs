use clap::Parser;
use std::collections::HashMap;

mod git;
mod optimization;
mod position_parser;
mod util;

type Error = Box<dyn std::error::Error>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(
        long,
        help = "Re-exports the graphs associated with the json run data in RE-EXPORT"
    )]
    re_export: Option<String>,

    #[clap(
        long,
        help = "Re-exports the graphs for all json files recursively in RE-EXPORT-DIR"
    )]
    re_export_all: Option<String>,

    #[clap(long, help = "Sets the prefix to use when exporting files")]
    re_export_prefix: Option<String>,

    /// Number of times to greet
    #[clap(
        long,
        help = "When enabled, checks out the commit that the program wants"
    )]
    use_git: bool,
}

fn main() {
    let args = Args::parse();

    let path = "NS3".to_owned();
    if args.use_git {
        let url = "https://github.com/TroyNeubauer/NS3NonIdealConditions2021.git";
        let needs_configure = match git::setup_repo(&git::RepoInfo {
            url: url.to_owned(),
            path: path.to_owned(),
            commit_hash: "ba8ea4ac58eada9679146ba2dc755789bbfbe91e".to_owned(),
        }) {
            Ok(needs_configure) => needs_configure,
            Err(err) => {
                eprintln!("Error while setting up repo: {}", err);
                return;
            }
        };
        if needs_configure {
            println!("Running configure");
            util::run_waf_command(
                &path,
                "configure --build-profile=optimized",
                map!("CXXFLAGS" => "-Wall"),
            )
            .unwrap();
        }
    }

    if let Some(file_path) = args.re_export {
        println!("Re-exporting data from {}", file_path);
        optimization::re_export(&file_path, args.re_export_prefix.as_deref())
            .expect("Failed to re-export data");
    } else if let Some(dir_path) = args.re_export_all {
        optimization::re_export_all(&dir_path)
            .expect("Failed to re-export data");
        
    } else {
        util::run_waf_command(&path, "build", HashMap::new()).expect("failed to build waf");

        optimization::run(&path);
    }
}
