// Frozen types-mode special-member declaration responsibility regression.
struct C{~C(){Word n;}typedef int Word;};
