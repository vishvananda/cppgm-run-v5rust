// N3485 6.4p1, 6.5p2: a declaration used as an if/loop substatement is
// implicitly a block; its object is destroyed at the end of each execution;
// expect exit 0
int destroyed;
struct S { ~S() { ++destroyed; } };
int main() { for (int i = 0; i < 3; ++i) S a; if (destroyed == 3) S b; return destroyed == 4 ? 0 : 1; }
