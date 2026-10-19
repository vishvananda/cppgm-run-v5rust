// N3485 15p2: a jump cannot enter a handler's block; expect reject
void f() {
  goto inside;
  try {} catch (...) { inside:; }
}
int main() { return 0; }
