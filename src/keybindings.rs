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
		KeyPress::Ctrl('H'),
		Cmd::Kill(Movement::BackwardWord(1, Word::Emacs)),
	);
	editor.bind_sequence(
		KeyPress::Ctrl('L'),
		Cmd::Kill(Movement::ForwardWord(1, At::Start, Word::Emacs)),
	);

	editor.bind_sequence(
		KeyPress::Ctrl('V'),
		Cmd::Yank(1, Anchor::After)
	);
}