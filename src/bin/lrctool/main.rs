use clap::{Arg, Command, value_parser};
use std::fs;
use std::io::{self};
use regex::Regex;

#[derive(Debug)]
struct SubtitleEntry {
    timestamp: String,
    username: String,
    content: String,
    start_ms: u32,
    end_ms: Option<u32>,
}

fn main() -> io::Result<()> {
    let matches = Command::new("lrctool")
        .version("0.1.0")
        .about("LRC 转 SRT/ASS 字幕工具")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .help("输入 LRC 文件路径")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("输出字幕文件路径")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .help("输出格式: srt 或 ass")
                .value_parser(["srt", "ass"]),
        )
        .get_matches();

    let input_file = matches.get_one::<String>("input").unwrap();
    let output_file = matches.get_one::<String>("output").map(|s| s.as_str());
    let format = matches.get_one::<String>("format").map(|s| s.as_str());

    // 读取LRC文件
    let content = fs::read_to_string(input_file)?;
    // 解析LRC内容
    let entries = parse_lrc(&content);

    // 决定输出格式和输出文件名
    let (output_format, output_path) = match (format, output_file) {
        (Some(fmt), Some(out)) => (fmt, out.to_string()),
        (Some(fmt), None) => {
            let ext = format!(".{}", fmt);
            let out = if input_file.ends_with(".lrc") {
                input_file.trim_end_matches(".lrc").to_string() + &ext
            } else {
                input_file.to_string() + &ext
            };
            (fmt, out)
        },
        (None, Some(out)) => {
            if out.ends_with(".ass") {
                ("ass", out.to_string())
            } else {
                ("srt", out.to_string())
            }
        },
        (None, None) => {
            // 默认 srt
            let out = if input_file.ends_with(".lrc") {
                input_file.trim_end_matches(".lrc").to_string() + ".srt"
            } else {
                input_file.to_string() + ".srt"
            };
            ("srt", out)
        }
    };

    match output_format {
        "ass" => {
            let ass_content = convert_to_ass(&entries);
            fs::write(&output_path, ass_content)?;
            println!("已生成ASS字幕文件: {}", output_path);
        },
        "srt" => {
            let srt_content = convert_to_srt(&entries);
            fs::write(&output_path, srt_content)?;
            println!("已生成SRT字幕文件: {}", output_path);
        },
        _ => {
            eprintln!("不支持的输出格式: {}，支持的格式: ass, srt", output_format);
        }
    }
    Ok(())
}

fn parse_lrc(content: &str) -> Vec<SubtitleEntry> {
    let re = Regex::new(r"\[(\d{2}):(\d{2}):(\d{2})\.(\d{3})\]([^\t]+)\t(.+)").unwrap();
    let mut entries = Vec::new();
    
    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            let minutes: u32 = caps[1].parse().unwrap_or(0);
            let seconds: u32 = caps[2].parse().unwrap_or(0);
            let milliseconds: u32 = caps[3].parse().unwrap_or(0) * 10 + caps[4].parse().unwrap_or(0);
            
            let total_ms = minutes * 60000 + seconds * 1000 + milliseconds;
            
            let entry = SubtitleEntry {
                timestamp: format!("{}:{}:{}.{}", &caps[1], &caps[2], &caps[3], &caps[4]),
                username: caps[5].trim().to_string(),
                content: caps[6].trim().to_string(),
                start_ms: total_ms,
                end_ms: None,
            };
            
            entries.push(entry);
        }
    }
    
    // 设置结束时间（下一条字幕的开始时间，或者+3秒作为默认）
    for i in 0..entries.len() {
        if i + 1 < entries.len() {
            entries[i].end_ms = Some(entries[i + 1].start_ms);
        } else {
            entries[i].end_ms = Some(entries[i].start_ms + 3000); // 最后一条显示3秒
        }
    }
    
    entries
}

fn convert_to_ass(entries: &[SubtitleEntry]) -> String {
    let mut ass_content = String::new();
    
    // ASS文件头
    ass_content.push_str("[Script Info]\n");
    ass_content.push_str("Title: LRC转换字幕\n");
    ass_content.push_str("ScriptType: v4.00+\n");
    ass_content.push_str("\n");
    
    // 样式定义
    ass_content.push_str("[V4+ Styles]\n");
    ass_content.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n");
    ass_content.push_str("Style: Default,Arial,16,&Hffffff,&Hffffff,&H0,&H0,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1\n");
    ass_content.push_str("\n");
    
    // 事件
    ass_content.push_str("[Events]\n");
    ass_content.push_str("Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n");
    
    for entry in entries {
        let start_time = ms_to_ass_time(entry.start_ms);
        let end_time = ms_to_ass_time(entry.end_ms.unwrap_or(entry.start_ms + 3000));
        
        ass_content.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{{\\c&H00ffff&}}{}{{\\c&Hffffff&}}: {}\n",
            start_time, end_time, entry.username, entry.content
        ));
    }
    
    ass_content
}

fn convert_to_srt(entries: &[SubtitleEntry]) -> String {
    let mut srt_content = String::new();
    
    for (index, entry) in entries.iter().enumerate() {
        let start_time = ms_to_srt_time(entry.start_ms);
        let end_time = ms_to_srt_time(entry.end_ms.unwrap_or(entry.start_ms + 3000));
        
        srt_content.push_str(&format!("{}\n", index + 1));
        srt_content.push_str(&format!("{} --> {}\n", start_time, end_time));
        srt_content.push_str(&format!("{}: {}\n\n", entry.username, entry.content));
    }
    
    srt_content
}

fn ms_to_ass_time(ms: u32) -> String {
    let hours = ms / 3600000;
    let minutes = (ms % 3600000) / 60000;
    let seconds = (ms % 60000) / 1000;
    let centiseconds = (ms % 1000) / 10;
    
    format!("{}:{:02}:{:02}.{:02}", hours, minutes, seconds, centiseconds)
}

fn ms_to_srt_time(ms: u32) -> String {
    let hours = ms / 3600000;
    let minutes = (ms % 3600000) / 60000;
    let seconds = (ms % 60000) / 1000;
    let milliseconds = ms % 1000;
    
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, milliseconds)
}