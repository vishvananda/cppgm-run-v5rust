// N3485 15.3p1: a handler cannot declare an rvalue reference; expect reject
void f() {
  try {} catch (int &&value) {}
}
int main() { return 0; }
