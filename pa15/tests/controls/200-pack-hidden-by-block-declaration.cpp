// N3485 3.3.10p1, 14.5.3p5: a block-scope int args hides the function
// parameter pack args, so (args + sizeof(Ts))... expands only Ts and uses the
// local; expect exit 0
template<class... Ts> int block(Ts... args) { int r = 0; { int args = 10; int a[] = { (args + (int)sizeof(Ts))... }; r = a[0] + a[1]; } return r; }
int main() { return block(1, 2) == 28 ? 0 : 1; }
