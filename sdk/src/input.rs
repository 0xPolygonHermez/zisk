use crate::hints::ZiskHints;
use crate::stdin::ZiskStdin;

/// Input to a [`prove`](crate::ProverClient::prove) or [`execute`](crate::ProverClient::execute)
/// operation.
///
/// Pass a [`ZiskStdin`] for normal execution, or [`ZiskHints`] for hint-driven execution
/// (Assembly executor required for hints).
///
/// Both types convert automatically via [`From`]:
/// ```ignore
/// client.prove(&PROGRAM, stdin)  // ZiskStdin → ProgramInput::Stdin
/// client.prove(&PROGRAM, hints)  // ZiskHints → ProgramInput::Hints
/// client.execute(&PROGRAM, stdin)
/// client.execute(&PROGRAM, hints)
/// ```
pub enum ProgramInput {
    /// Standard in-memory or file-backed input.
    Stdin(ZiskStdin),
    /// Hints stream. Assembly executor required.
    Hints(ZiskHints),
}

impl From<ZiskStdin> for ProgramInput {
    fn from(stdin: ZiskStdin) -> Self {
        Self::Stdin(stdin)
    }
}

impl From<ZiskHints> for ProgramInput {
    fn from(hints: ZiskHints) -> Self {
        Self::Hints(hints)
    }
}
