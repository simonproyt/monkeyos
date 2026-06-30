use std::env;
use std::ffi::OsString;

fn main() {
    let args: Vec<OsString> = env::args_os().collect();
    if args.is_empty() {
        return;
    }

    if let Ok(pwd) = env::var("PWD") {
        let _ = env::set_current_dir(&pwd);
    }

    let program_path = args[0].to_string_lossy();
    let program = program_path.split('/').next_back().unwrap_or(&program_path);

    match program {
        "ls" => {
            let _ = uu_ls::uumain(args.into_iter());
        }
        "cat" => {
            let _ = uu_cat::uumain(args.into_iter());
        }
        "echo" => {
            let _ = uu_echo::uumain(args.into_iter());
        }
        "mkdir" => {
            let _ = uu_mkdir::uumain(args.into_iter());
        }
        "rm" => {
            let _ = uu_rm::uumain(args.into_iter());
        }
        "touch" => {
            let _ = uu_touch::uumain(args.into_iter());
        }
        "pwd" => {
            let _ = uu_pwd::uumain(args.into_iter());
        }
        "wc" => {
            let _ = uu_wc::uumain(args.into_iter());
        }
        "sort" => {
            let _ = uu_sort::uumain(args.into_iter());
        }
        "head" => {
            let _ = uu_head::uumain(args.into_iter());
        }
        "tail" => {
            let _ = uu_tail::uumain(args.into_iter());
        }
        _ => {
            eprintln!("{}: command not found in coreutils multicall binary", program);
        }
    }
}
