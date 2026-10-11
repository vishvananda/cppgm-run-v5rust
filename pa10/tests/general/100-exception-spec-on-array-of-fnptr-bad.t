// N3485 15.4p2: an exception-specification may appear only on a
// function/pointer-to-function declarator that is the top-level type; an array
// of pointers to function with throw(int) must be rejected
void g1() throw(int);
void (*fps[2])() throw(int) = { g1, g1 };
int main() { return 0; }
