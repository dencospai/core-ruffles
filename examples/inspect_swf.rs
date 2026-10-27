//! Inspect SWF metadata and actions. Pass --all to include nested timelines/buttons.
use std::{env, fs};
use swf::avm1::{read::Reader, types::Action};

fn actions(bytes: &[u8], version: u8, indent: usize) {
    let mut reader = Reader::new(bytes, version);
    while !reader.get_ref().is_empty() {
        let offset = bytes.len() - reader.get_ref().len();
        let action = reader.read_action().unwrap();
        match &action {
            Action::DefineFunction(f) => {
                println!("{:indent$}{offset}: Function {:?}", "", f.name);
                actions(f.actions, version, indent + 2);
            }
            Action::DefineFunction2(f) => {
                println!("{:indent$}{offset}: Function2 {:?}", "", f.name);
                actions(f.actions, version, indent + 2);
            }
            _ => println!("{:indent$}{offset}: {action:?}", ""),
        }
    }
}

fn main() {
    let data = fs::read(env::args().nth(1).expect("SWF path")).unwrap();
    let buffer = swf::decompress_swf(&data[..]).unwrap();
    let movie = swf::parse_swf(&buffer).unwrap();
    println!("{:?}", movie.header);
    if env::args().any(|arg| arg == "--all") {
        tags(&movie.tags, movie.header.version(), "root");
        return;
    }
    for tag in movie.tags {
        if let swf::Tag::DoAction(data) = tag {
            actions(data, movie.header.version(), 0);
            break;
        }
    }
}

fn tags(tags: &[swf::Tag<'_>], version: u8, scope: &str) {
    let mut frame = 1;
    for tag in tags {
        match tag {
            swf::Tag::ShowFrame => frame += 1,
            swf::Tag::FrameLabel(label) => println!("{scope} frame {frame}: label {label:?}"),
            swf::Tag::DoAction(data) => {
                println!("{scope} frame {frame}: actions");
                actions(data, version, 2);
            }
            swf::Tag::DefineSprite(sprite) => {
                tags_in_sprite(sprite, version, scope);
            }
            swf::Tag::DefineButton(button) | swf::Tag::DefineButton2(button) => {
                for action in &button.actions {
                    println!("{scope} button {}: {:?}", button.id, action.conditions);
                    actions(action.action_data, version, 2);
                }
            }
            swf::Tag::PlaceObject(object) => {
                if let Some(actions_list) = &object.clip_actions {
                    for action in &actions_list.records {
                        println!("{scope} frame {frame} clip: {:?}", action.events);
                        actions(action.action_data, version, 2);
                    }
                }
            }
            _ => (),
        }
    }
}

fn tags_in_sprite(sprite: &swf::Sprite<'_>, version: u8, scope: &str) {
    tags(
        &sprite.tags,
        version,
        &format!("{scope}/sprite{}", sprite.id),
    );
}
