// N3485 4.7: bool truth values cross signed and unsigned integer results;
// expect exit 0
long signed_result(int* p) { return p == 0; }
unsigned long unsigned_result(int n) { return n < 3; }
long stored_result(bool value) { return value; }
int main() {
  return signed_result(0) == 1 && unsigned_result(4) == 0 && stored_result(true) == 1 ? 0 : 1;
}
