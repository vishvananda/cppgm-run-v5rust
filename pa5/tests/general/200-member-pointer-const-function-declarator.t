struct C { int f(int) const; };

typedef int (C::*pmf)(int) const;
