use rustyline::{At, Cmd, Editor, KeyPress, Movement, Word, Anchor};

pub fn set_keybindings(editor: &mut Editor<()>) {
	editor.bind_sequence(
		KeyPress::ControlLeft,
		Cmd::Move(Movement::BackwardWord(1, Word::Emacs)),
	);
	editor.bind_sequence(
		KeyPress::ControlRight,
		Cmd::Move(Movement::ForwardWord(1, At::Start, Word::Emacs)),
	);

	editor.bind_sequence(
		KeyPress::Ctrl('Z'),
		Cmd::Undo(1)
	);
	editor.bind_sequence(
		KeyPress::Ctrl('V'),
		Cmd::Yank(1, Anchor::After)
	);

	editor.bind_sequence(
		KeyPress::Ctrl('\x08'),
		Cmd::Kill(Movement::BackwardWord(1, Word::Emacs)),
	);
	editor.bind_sequence(
		KeyPress::Ctrl('\x7f'),
		Cmd::Kill(Movement::BackwardWord(1, Word::Emacs)),
	);
}