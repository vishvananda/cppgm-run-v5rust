struct P { operator int* const&() const; };
void f(P p) { delete p; }
