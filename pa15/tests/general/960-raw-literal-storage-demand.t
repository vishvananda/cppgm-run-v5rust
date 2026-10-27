int operator""_raw(const char* text) {
  return text[0] == '1' ? 0 : 1;
}
int read_raw() {
  return 18446744073709551616_raw + 1e999999999_raw;
}
int main() { return read_raw(); }
