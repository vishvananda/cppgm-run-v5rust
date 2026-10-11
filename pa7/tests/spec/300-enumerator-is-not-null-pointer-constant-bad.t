// N3485 focus: 4.10 [conv.ptr] only an integer literal zero is a null pointer constant
enum { zero = 0 };
void take(int *);
void test() { take(zero); }
