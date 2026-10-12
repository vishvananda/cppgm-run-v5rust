// Frozen types-mode special-member declaration responsibility regression.
struct C{operator Word(){return 1;}typedef int Word;};
