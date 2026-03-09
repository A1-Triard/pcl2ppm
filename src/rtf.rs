use crate::pcl::PclCommand;
use either::{Left, Right};
use oem_cp::Cp866;

#[derive(Debug)]
pub struct Rtf {
    lines: Vec<String>,
    left_margin: u32,
    top_margin: u32,
}

#[derive(Debug)]
pub enum PclToRtfError {
    UnexpectedEnd,
    TextInPreamble(u32),
    UnexpectedCommand(u32),
}

pub fn pcl_to_rtf(pcl: &mut dyn Iterator<Item=(PclCommand, u32)>) -> Result<Rtf, PclToRtfError> {
    enum State { Preamble, LeftMarginSet, TopMarginSet, Text(bool) }
    let mut rtf = Rtf { lines: Vec::new(), left_margin: 0, top_margin: 0 };
    let mut state = State::Preamble;
    loop {
        match state {
            State::Preamble => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::LineTermination(0) => { },
                    PclCommand::ClearHorizontalMargins => { },
                    PclCommand::VerticalMotionIndex(_) => { },
                    PclCommand::RasterGraphicsPresentationMode(_) => { },
                    PclCommand::EndOfLineWrap(_) => { },
                    PclCommand::SecondarySymbolSet(8300, 88) => { },
                    PclCommand::VerticalCursorPositioning(Left(0)) => { },
                    PclCommand::Char(c) if c >= b' ' => return Err(PclToRtfError::TextInPreamble(offset)),
                    PclCommand::Char(13) => { },
                    PclCommand::Char(14) => { },
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.top_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::TopMarginSet;
                    },
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.left_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::LeftMarginSet;
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::LeftMarginSet => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::VerticalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.top_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::Text(true);
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::TopMarginSet => {
                let (command, offset) = pcl.next().ok_or(PclToRtfError::UnexpectedEnd)?;
                match command {
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x >= 0 => {
                        rtf.left_margin = u32::try_from(x).unwrap() * 24 / 5;
                        state = State::Text(true);
                    },
                    _ => return Err(PclToRtfError::UnexpectedCommand(offset)),
                }
            },
            State::Text(new_line) => {
                let Some((command, offset)) = pcl.next() else { return Ok(rtf); };
                match command {
                    PclCommand::Char(c) if c >= b' ' => {
                        let c = char::from(Cp866(c));
                        if new_line {
                            rtf.lines.push(String::new());
                        }
                        rtf.lines.last_mut().unwrap().push(c);
                    },
                    PclCommand::HorizontalCursorPositioning(Right(x)) if x != 0 && x % 45 == 0 => {
                        if new_line {
                            rtf.lines.push(String::new());
                        }
                        for _ in 0 .. x / 45 {
                            rtf.lines.last_mut().unwrap().push(' ');
                        }
                    },
                    _ => {
                        eprintln!("{command:?}");
                        return Err(PclToRtfError::UnexpectedCommand(offset));
                    },
                }
            },
        }
    }
}
