// Frozen types-mode special-member declaration responsibility regression.
struct C{typedef int I;operator I();};C::operator I(){return 1;}
