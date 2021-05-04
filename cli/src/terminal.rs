/*
 * Created on Tue May 04 2021
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2021, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 * 
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

use crossterm::cursor;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal;
use crossterm::{Command, ExecutableCommand};
use std::error::Error;
use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::Stdout;
use std::process;
use std::slice;
use std::slice::SliceIndex;
use terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};

type DynError = Box<dyn Error>;
type EmptyRetError = Result<(), DynError>;
const TERMINAL_BUFFER_SIZE: usize = 128;

pub struct Terminal {
    stdout: Stdout,
    internal_buffer: String,
    history: Vec<String>,
    bytes_left_to_go_ahead_on_screen: usize,
    bytes_left_to_go_back_on_screen: usize,
    current_index: Option<usize>,
    bytes_from_history: usize,
    init_len: usize,
    cursor_position: (u16, u16),
}

impl Terminal {
    pub fn new(stdout: Stdout) -> Result<Self, DynError> {
        let history = fs::read_to_string(".sky_history")
            .map(|vals| {
                vals.lines()
                    .map(|val| val.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or(Vec::new());
        enable_raw_mode()?;
        Ok(Terminal {
            stdout,
            internal_buffer: String::with_capacity(TERMINAL_BUFFER_SIZE),
            bytes_left_to_go_ahead_on_screen: 0,
            bytes_left_to_go_back_on_screen: 0,
            bytes_from_history: 0,
            current_index: None,
            init_len: history.len(),
            history,
            cursor_position: cursor::position()?,
        })
    }
    fn update_internal_cursor(&mut self) -> EmptyRetError {
        self.cursor_position = cursor::position()?;
        Ok(())
    }
    pub fn run_repl(mut self) {
        fn run(terminal: &mut Terminal) -> Result<(), DynError> {
            terminal.print_skysh()?;
            loop {
                match read()? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char(ch),
                        modifiers: KeyModifiers::NONE,
                    })
                    | Event::Key(KeyEvent {
                        code: KeyCode::Char(ch),
                        modifiers: KeyModifiers::SHIFT,
                    }) => terminal.read_char(ch)?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::NONE,
                    }) => {
                        if terminal.run_on_enter()? {
                            break Ok(());
                        }
                    }
                    Event::Key(KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::NONE,
                    }) => terminal.history_scroll_up()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers::NONE,
                    }) => terminal.history_scroll_down()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::NONE,
                    }) => terminal.run_backspace()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Right,
                        modifiers: KeyModifiers::NONE,
                    }) => terminal.terminal_scroll_right()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Left,
                        modifiers: KeyModifiers::NONE,
                    }) => terminal.terminal_scroll_left()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Left,
                        modifiers: KeyModifiers::CONTROL,
                    }) => terminal.terminal_scroll_left_end()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Right,
                        modifiers: KeyModifiers::CONTROL,
                    }) => terminal.terminal_scroll_right_end()?,
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                    }) => return Ok(()),
                    Event::Resize(x, y) => terminal.resize_terminal(x, y)?,
                    _ => {
                        // Ignore all unknown keystrokes
                        continue;
                    }
                }
            }
        }
        if let Err(e) = run(&mut self) {
            eprintln!("Error while running REPL: '{}'", e);
            process::exit(0x100);
        }
        if let Err(e) = self.save_history_and_terminate() {
            eprintln!("Error while terminating REPL and saving history: '{}'", e);
            process::exit(0x100);
        }
    }
    fn print_skysh(&mut self) -> EmptyRetError {
        self.writeln("skysh> ")
    }
    fn push_to_buf(&mut self, ch: char) {
        self.internal_buffer.push(ch);
    }
    fn insert_into_buf(&mut self, idx: usize, ch: char) {
        self.internal_buffer.insert(idx, ch)
    }
    fn writeln(&mut self, dat: impl Display) -> EmptyRetError {
        // First set cursor to (col: 0, row: next line)
        self.run(cursor::MoveToColumn(0))?;
        self.terminal_scroll_down_if_required()?;
        self.run(cursor::MoveToNextLine(1))?;
        self.run(Print(dat))
    }
    fn terminal_scroll_down_if_required(&mut self) -> EmptyRetError {
        let (current_column, current_height) = self.cursor_position;
        let (_terminal_width, terminal_height) = terminal::size()?;
        if current_height + 1 >= terminal_height {
            self.run(terminal::ScrollUp(1))?;
            self.run(cursor::MoveToColumn(current_column))?;
        }
        Ok(())
    }
    fn write(&mut self, dat: impl Display) -> EmptyRetError {
        self.run(Print(dat))
    }
    fn run(&mut self, cmd: impl Command) -> EmptyRetError {
        self.stdout.execute(cmd)?;
        self.update_internal_cursor()?;
        Ok(())
    }
    fn cursor_has_moved(&self) -> bool {
        self.bytes_left_to_go_ahead_on_screen != 0
            || self.bytes_left_to_go_back_on_screen != self.internal_buffer.len()
    }
    fn read_char(&mut self, ch: char) -> EmptyRetError {
        if self.cursor_has_moved() {
            // Say we have 'ABCDEF'
            // Say our cursor is at: 'ABC|D|EF'
            // So bytes left to go back = 3
            // And if we insert a char here, then our insert position in last_command will also be
            // 3
            self.insert_into_buf(self.bytes_left_to_go_back_on_screen, ch);
            self.save_cursor_position()?;
            // Now we need to clear bytes_left_to_go_ahead_on_screen and update it
            // First, position the cursor at the insert point - 1 (since the insert item needs to be
            // updated itself)
            /*
            So, assuming we inserted C in the position, our resulting sequence will look like:
            'ABCC|D|EF' with the cursor at D. We need to update everything from the insert_position+1
            to the end of last_command.
            */
            // Now clear the remaining chars; our cursor is already at the point from where we need to clear
            self.clear_from_cursor_down()?;
            // Now print the remaining command
            self.write_idx(self.bytes_left_to_go_back_on_screen..)?;
            self.restore_cursor_position()?; // get back to the old position
            self.cursor_move_right(1)?; // the next char goes beyond this point; so, move the cursor ahead
        } else {
            self.write(ch)?;
            self.push_to_buf(ch);
        }
        self.bytes_left_to_go_back_on_screen += 1;
        Ok(())
    }
    fn run_on_enter(&mut self) -> Result<bool, DynError> {
        // Since we're executing, we can clean anything that previously existed
        self.current_index = None;
        // Reset as we're executing this and clearing anything
        self.bytes_left_to_go_back_on_screen = 0;
        self.bytes_left_to_go_ahead_on_screen = 0;
        if self.internal_buffer.len() != 0 {
            match self.internal_buffer.to_lowercase().as_str() {
                "exit" => return Ok(true),
                "clear" => {
                    self.internal_buffer.clear();
                    self.clear_terminal()?;
                    self.print_skysh()?;
                    self.save_cursor_position()?;
                    return Ok(false);
                }
                _ => (),
            }
            self.writeln(format!("Executing: `{}`", self.internal_buffer))?;
            if let Some(true) = self
                .history
                .last()
                .map(|last_stored_in_history| last_stored_in_history != &self.internal_buffer)
            {
                // The last command in history is also the same as the current last_command
                // So we'll ignore it and insert if it isn't the same as the last command
                self.append_to_history(self.internal_buffer.clone());
            } else if self.history.len() == 0 {
                // There was no previously stored command, ignore it
                self.append_to_history(self.internal_buffer.clone());
            }
            self.internal_buffer.clear();
            self.bytes_from_history = 0;
        }
        self.print_skysh()?;
        Ok(false)
    }
    fn history_scroll_up(&mut self) -> EmptyRetError {
        if self.history.len() != 0 {
            self.cursor_move_left(self.bytes_left_to_go_back_on_screen as u16)?;
            // Now clear these bytes
            self.clear_from_cursor_down()?;
            // So the new index will be the last item (since we're going UP from HIGH to LOW)
            let new_index = self
                .current_index
                .map(|idx| {
                    // we need to go one back from the current index
                    if idx == 0 {
                        // This is the oldest entry; stay here
                        idx
                    } else {
                        idx - 1
                    }
                })
                .unwrap_or(self.history.len() - 1);
            // Set current_index to new_index
            self.current_index = Some(new_index);
            let ret = &self.history[new_index];
            self.internal_buffer = ret.clone();
            self.bytes_from_history = ret.len();
            self.bytes_left_to_go_back_on_screen = ret.len();
            self.bytes_left_to_go_ahead_on_screen = 0;
            // Write the command
            self.write_idx(..)?;
        }
        Ok(())
    }
    fn history_scroll_down(&mut self) -> EmptyRetError {
        if self.history.len() != 0 {
            self.cursor_move_left(self.bytes_left_to_go_back_on_screen as u16)?;
            // Now clear these bytes
            self.clear_from_cursor_down()?;
            // We're going DOWN from HIGH to LOW)
            let ret: String = self
                .current_index
                .map(|idx| {
                    // we need to go one back from the current index
                    let our_idx = idx + 1;
                    if our_idx >= self.history.len() {
                        // We've already showed the last item
                        // and yet this fella wants to go down more; whatcha lookin' for, eh?
                        /*
                        We set current_index to None because we've already shown our_idx and we're just
                        returning an empty string (after the final unwrap_or). So the next time the person
                        attempts a scroll up, they don't start at the last counter
                        */
                        self.current_index = None;
                        None
                    } else {
                        // Still in bounds; go ahead
                        self.current_index = Some(our_idx);
                        self.history.get(our_idx)
                    }
                })
                .unwrap_or(None)
                .unwrap_or(&"".to_owned())
                .to_string();
            self.internal_buffer = ret.clone();
            self.bytes_from_history = ret.len();
            self.bytes_left_to_go_back_on_screen = ret.len();
            self.bytes_left_to_go_ahead_on_screen = 0;
            // Write the command
            self.write(&ret)?;
        }
        Ok(())
    }
    fn run_backspace(&mut self) -> EmptyRetError {
        if self.internal_buffer.len() != 0 && self.bytes_left_to_go_back_on_screen != 0 {
            if self.cursor_has_moved() && self.bytes_left_to_go_back_on_screen != 0 {
                /*
                So the cursor has moved and just popping off from the extreme right won't work
                If we have 'ABC|D|E' with the cursor at D, a backspace should remove C. Similarly,
                if we have 'AB|C|DEF' witht the cursor at C, then it should remove B.
                In case 1: position to remove = 3; idx = 2
                In case 2: position to remove = 2; idx = 1
                In case 1: bytes left to go back = 3 => idx = bytes left to go back - 1 = 2
                In case 2: bytes left to go back = 2 => idx = bytes left to go back - 1 = 1
                Therefore we need to remove this idx
                */
                let _ = self
                    .internal_buffer
                    .remove(self.bytes_left_to_go_back_on_screen - 1);
                /*
                We'll assume the first case: 'ABC|D|E'; now we removed 'C' from our internal string
                but not from the terminal! The first thing to do would be to move back the cursor to
                C and then clear everything from that point
                */
                self.cursor_move_left(1)?;
                self.clear_from_cursor_down()?;
                // We'll save the cursor position as we'll need it in the coming steps
                self.save_cursor_position()?;
                /*
                Now we have 'AB' on the terminal and nothing ahead. So print out everything from the index
                that we removed! In our case, ABDE is the internal string, so we need to print from idx 2
                all the way to the end; let's do this
                */
                self.write_idx(self.bytes_left_to_go_back_on_screen - 1..)?;
                /*
                So our terminal has the following look: 'ABDE||' with the cursor at the end; whoa, that's
                wrong! It should be at D!
                */
                self.restore_cursor_position()?;
                /*
                So our terminal now looks like: 'AB|D|E'; we can go ahead by one and go back by 2
                */
            } else {
                if self.bytes_left_to_go_back_on_screen != 0 {
                    let _ = self.internal_buffer.pop(); // remove the last character
                    self.cursor_move_left(1)?; // move the cursor back
                    self.clear_from_cursor_down()?; // now clear everything beyond this point
                }
            }
            if self.bytes_left_to_go_back_on_screen != 0 {
                self.bytes_left_to_go_back_on_screen -= 1;
            };
            if self.bytes_from_history != 0 {
                // So we've printed some line that has history and we're trying to edit that
                // make sure that we reduce this counter so that when we scroll UP/DOWN through
                // history, we don't end up clearing out the `skysh> ` prompt
                self.bytes_from_history -= 1;
            }
        }
        Ok(())
    }
    fn append_to_history(&mut self, item: String) {
        self.history.push(item);
    }
    fn write_idx(&mut self, idx: impl SliceIndex<[u8], Output = [u8]>) -> EmptyRetError {
        // We do this to avoid borrowed as mutable/immutable errors
        let s = unsafe {
            let len = self.internal_buffer.len();
            let ptr = self.internal_buffer.as_ptr();
            // Manually assemble the string slice
            let slice = &slice::from_raw_parts(ptr, len);
            // We already know it is utf-8, so just write it in!
            std::str::from_utf8_unchecked(&slice[idx])
        };
        self.write(s)?;
        Ok(())
    }
    fn _terminal_scroll_right(&mut self, n: u16) -> EmptyRetError {
        if self.bytes_left_to_go_ahead_on_screen != 0 {
            // As we moved a byte ahead, we can go back by one more byte
            self.bytes_left_to_go_ahead_on_screen -= n as usize;
            self.bytes_left_to_go_back_on_screen += n as usize;
            self.cursor_move_right(n)?;
        }
        Ok(())
    }
    fn _terminal_scroll_left(&mut self, n: u16) -> EmptyRetError {
        if self.bytes_left_to_go_back_on_screen != 0 {
            // So we do have some bytes on screen
            // Since we're going back by one byte, we can go ahead by one more byte
            self.bytes_left_to_go_back_on_screen -= n as usize;
            self.bytes_left_to_go_ahead_on_screen += n as usize;
            // Now move the cursor back
            self.cursor_move_left(n)?;
        }
        Ok(())
    }
    fn resize_terminal(&mut self, x: u16, y: u16) -> EmptyRetError {
        self.run(terminal::SetSize(x, y))
    }
    fn terminal_scroll_left(&mut self) -> EmptyRetError {
        self._terminal_scroll_left(1)
    }
    fn terminal_scroll_right(&mut self) -> EmptyRetError {
        self._terminal_scroll_right(1)
    }
    fn terminal_scroll_left_end(&mut self) -> EmptyRetError {
        self._terminal_scroll_left(self.bytes_left_to_go_back_on_screen as u16)
    }
    fn terminal_scroll_right_end(&mut self) -> EmptyRetError {
        self._terminal_scroll_right(self.bytes_left_to_go_ahead_on_screen as u16)
    }
    fn clear_from_cursor_down(&mut self) -> EmptyRetError {
        self.run(Clear(ClearType::FromCursorDown))
    }
    fn save_cursor_position(&mut self) -> EmptyRetError {
        self.run(cursor::SavePosition)
    }
    fn restore_cursor_position(&mut self) -> EmptyRetError {
        self.run(cursor::RestorePosition)
    }
    fn cursor_move_right(&mut self, n: u16) -> EmptyRetError {
        self.run(cursor::MoveRight(n))
    }
    fn cursor_move_left(&mut self, n: u16) -> EmptyRetError {
        self.run(cursor::MoveLeft(n))
    }
    fn clear_terminal(&mut self) -> EmptyRetError {
        // Clean the screen
        self.run(Clear(ClearType::All))?;
        // Move the cursor to the first position
        self.run(cursor::MoveTo(0, 0))
    }
    fn write_goodbye() {
        println!("\nGoodbye!");
    }
    fn save_history_and_terminate(self) -> EmptyRetError {
        let Terminal {
            history, init_len, ..
        } = self;
        disable_raw_mode()?;
        Self::write_goodbye();
        use std::io::Write;
        if history.len() != init_len {
            // Only write to history if the user has executed anything in this session
            OpenOptions::new()
                .create(true)
                .append(true)
                .truncate(false)
                .open(".sky_history")?
                .write_all(
                    &history
                        .into_iter()
                        .skip(init_len) // skip the previous entries
                        .map(|string| string + "\n")
                        .map(|string| string.into_bytes())
                        .flatten()
                        .collect::<Vec<u8>>(),
                )
                .map_err(|e| format!("Failed to write history with error: '{}'", e))?;
        }
        Ok(())
    }
}
