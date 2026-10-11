// N3485 5.3.4p5, 8.3.4: new E2[2] with typedef E E2[2] allocates an array of 4
// E (count in the cookie) and delete[] destroys all 4; expect exit 0
static int dtors = 0;
struct E { int v; E() : v(1) {} ~E() { ++dtors; } };
typedef E E2[2];
int main() { E2* p = new E2[2]; p[1][1].v = 5; delete[] p; return dtors == 4 ? 0 : 1; }
