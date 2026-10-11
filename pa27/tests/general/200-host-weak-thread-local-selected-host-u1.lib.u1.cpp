// GNU weak attribute, N3485 3.6.2p2, 3.7.2: when a strong definition of a
// dynamically initialized thread_local variable in one unit replaces a weak
// one in another, the program has one object, initialized once by the strong
// definition's initializer, whichever unit uses it first
#include "200-host-weak-thread-local-selected-host-u1.helper.h"
int from_strong() { printf("strong init\n"); return 100; }
thread_local int w = from_strong();
int main() { int a = read_u2(); printf("u2 sees %d, u1 sees %d\n", a, w); return 0; }
