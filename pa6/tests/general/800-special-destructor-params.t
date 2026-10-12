// Frozen types-mode special-member declaration responsibility regression.
struct C{~C(int p){int n=p;}};
