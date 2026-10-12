// Frozen types-mode special-member declaration responsibility regression.
struct C{typedef int I;operator int();operator I();};
